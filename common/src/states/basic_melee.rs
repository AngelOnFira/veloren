use crate::{
    combat::{Attack, AttackDamage, AttackEffect, CombatBuff, CombatEffect, CombatRequirement},
    comp::{CharacterState, Melee, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    Damage, DamageSource, GroupTarget, Knockback, KnockbackDir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub base_damage: f32,
    /// Base poise reduction
    pub base_poise_damage: f32,
    /// Knockback
    pub knockback: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// What key is used to press ability
    pub ability_key: AbilityKey,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.7);
        handle_jump(data, &mut update);
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, false);
            match update.character {
                CharacterState::BasicMelee(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::BasicMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if !self.exhausted {
                    update.character = CharacterState::BasicMelee(Data {
                        timer: Duration::default(),
                        exhausted: true,
                        ..*self
                    });

                    let poise = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Poise(self.static_data.base_poise_damage as f32),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let knockback = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Knockback(Knockback {
                            strength: self.static_data.knockback,
                            direction: KnockbackDir::Away,
                        }),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let energy = AttackEffect::new(None, CombatEffect::EnergyReward(50.0))
                        .with_requirement(CombatRequirement::AnyDamage);
                    let buff = CombatEffect::Buff(CombatBuff::default_physical());
                    let damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Melee,
                            value: self.static_data.base_damage as f32,
                        },
                        Some(GroupTarget::OutOfGroup),
                    )
                    .with_effect(buff);
                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_crit(0.5, 1.3)
                        .with_effect(energy)
                        .with_effect(poise)
                        .with_effect(knockback);

                    // Hit attempt
                    data.updater.insert(data.entity, Melee {
                        attack,
                        range: self.static_data.range,
                        max_angle: self.static_data.max_angle,
                        applied: false,
                        hit_count: 0,
                    });
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::BasicMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::BasicMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<Melee>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Melee>(data.entity);
            },
        }

        update
    }
}
