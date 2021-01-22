use crate::{
    frame::Frame,
    message::{MessageBuffer, OutgoingMessage},
    metrics::{ProtocolMetricCache, RemoveReason},
    types::{Bandwidth, Mid, Prio, Promises, Sid},
};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Debug)]
struct StreamInfo {
    pub(crate) guaranteed_bandwidth: Bandwidth,
    pub(crate) prio: Prio,
    pub(crate) promises: Promises,
    pub(crate) messages: Vec<OutgoingMessage>,
}

/// Responsible for queueing messages.
/// every stream has a guaranteed bandwidth and a prio 0-7.
/// when `n` Bytes are available in the buffer, first the guaranteed bandwidth
/// is used. Then remaining bandwidth is used to fill up the prios.
#[derive(Debug)]
pub(crate) struct PrioManager {
    streams: HashMap<Sid, StreamInfo>,
    metrics: ProtocolMetricCache,
}

// Send everything ONCE, then keep it till it's confirmed

impl PrioManager {
    const HIGHEST_PRIO: u8 = 7;

    pub fn new(metrics: ProtocolMetricCache) -> Self {
        Self {
            streams: HashMap::new(),
            metrics,
        }
    }

    pub fn open_stream(
        &mut self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        guaranteed_bandwidth: Bandwidth,
    ) {
        self.streams.insert(sid, StreamInfo {
            guaranteed_bandwidth,
            prio,
            promises,
            messages: vec![],
        });
    }

    pub fn try_close_stream(&mut self, sid: Sid) -> bool {
        if let Some(si) = self.streams.get(&sid) {
            if si.messages.is_empty() {
                self.streams.remove(&sid);
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool { self.streams.is_empty() }

    pub fn add(&mut self, buffer: Arc<MessageBuffer>, mid: Mid, sid: Sid) {
        self.streams
            .get_mut(&sid)
            .unwrap()
            .messages
            .push(OutgoingMessage::new(buffer, mid, sid));
    }

    /// bandwidth might be extended, as for technical reasons
    /// guaranteed_bandwidth is used and frames are always 1400 bytes.
    pub fn grab(&mut self, bandwidth: Bandwidth, dt: Duration) -> Vec<Frame> {
        let total_bytes = (bandwidth as f64 * dt.as_secs_f64()) as u64;
        let mut cur_bytes = 0u64;
        let mut frames = vec![];

        let mut prios = [0u64; (Self::HIGHEST_PRIO + 1) as usize];
        let metrics = &self.metrics;

        let mut process_stream =
            |stream: &mut StreamInfo, mut bandwidth: i64, cur_bytes: &mut u64| {
                let mut finished = vec![];
                'outer: for (i, msg) in stream.messages.iter_mut().enumerate() {
                    while let Some(frame) = msg.next() {
                        let b = if matches!(frame, Frame::DataHeader { .. }) {
                            25
                        } else {
                            19 + OutgoingMessage::FRAME_DATA_SIZE
                        };
                        bandwidth -= b as i64;
                        *cur_bytes += b;
                        frames.push(frame);
                        if bandwidth <= 0 {
                            break 'outer;
                        }
                    }
                    finished.push(i);
                }

                //cleanup
                for i in finished.iter().rev() {
                    let msg = stream.messages.remove(*i);
                    let (sid, bytes) = msg.get_sid_len();
                    metrics.smsg_ot(sid, RemoveReason::Finished);
                    metrics.smsg_ob(sid, RemoveReason::Finished, bytes);
                }
            };

        // Add guaranteed bandwidth
        for (_, stream) in &mut self.streams {
            prios[stream.prio.min(Self::HIGHEST_PRIO) as usize] += 1;
            let stream_byte_cnt = (stream.guaranteed_bandwidth as f64 * dt.as_secs_f64()) as u64;
            process_stream(stream, stream_byte_cnt as i64, &mut cur_bytes);
        }

        if cur_bytes < total_bytes {
            // Add optional bandwidth
            for prio in 0..=Self::HIGHEST_PRIO {
                if prios[prio as usize] == 0 {
                    continue;
                }
                let per_stream_bytes = (total_bytes - cur_bytes) / prios[prio as usize];

                for (_, stream) in &mut self.streams {
                    if stream.prio != prio {
                        continue;
                    }
                    process_stream(stream, per_stream_bytes as i64, &mut cur_bytes);
                }
            }
        }

        frames
    }
}
