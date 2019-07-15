use crate::window::KeyMouse;
use directories::ProjectDirs;
use glutin::{MouseButton, VirtualKeyCode};
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, path::PathBuf};

/// `ControlSettings` contains keybindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    pub toggle_cursor: KeyMouse,
    pub escape: KeyMouse,
    pub enter: KeyMouse,
    pub command: KeyMouse,
    pub move_forward: KeyMouse,
    pub move_left: KeyMouse,
    pub move_back: KeyMouse,
    pub move_right: KeyMouse,
    pub jump: KeyMouse,
    pub glide: KeyMouse,
    pub map: KeyMouse,
    pub bag: KeyMouse,
    pub quest_log: KeyMouse,
    pub character_window: KeyMouse,
    pub social: KeyMouse,
    pub spellbook: KeyMouse,
    pub settings: KeyMouse,
    pub help: KeyMouse,
    pub toggle_interface: KeyMouse,
    pub toggle_debug: KeyMouse,
    pub fullscreen: KeyMouse,
    pub screenshot: KeyMouse,
    pub toggle_ingame_ui: KeyMouse,
    pub attack: KeyMouse,
    pub second_attack: KeyMouse,
    pub roll: KeyMouse,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            toggle_cursor: KeyMouse::Key(VirtualKeyCode::Tab),
            escape: KeyMouse::Key(VirtualKeyCode::Escape),
            enter: KeyMouse::Key(VirtualKeyCode::Return),
            command: KeyMouse::Key(VirtualKeyCode::Slash),
            move_forward: KeyMouse::Key(VirtualKeyCode::W),
            move_left: KeyMouse::Key(VirtualKeyCode::A),
            move_back: KeyMouse::Key(VirtualKeyCode::S),
            move_right: KeyMouse::Key(VirtualKeyCode::D),
            jump: KeyMouse::Key(VirtualKeyCode::Space),
            glide: KeyMouse::Key(VirtualKeyCode::LShift),
            map: KeyMouse::Key(VirtualKeyCode::M),
            bag: KeyMouse::Key(VirtualKeyCode::B),
            quest_log: KeyMouse::Key(VirtualKeyCode::L),
            character_window: KeyMouse::Key(VirtualKeyCode::C),
            social: KeyMouse::Key(VirtualKeyCode::O),
            spellbook: KeyMouse::Key(VirtualKeyCode::P),
            settings: KeyMouse::Key(VirtualKeyCode::N),
            help: KeyMouse::Key(VirtualKeyCode::F1),
            toggle_interface: KeyMouse::Key(VirtualKeyCode::F2),
            toggle_debug: KeyMouse::Key(VirtualKeyCode::F3),
            fullscreen: KeyMouse::Key(VirtualKeyCode::F11),
            screenshot: KeyMouse::Key(VirtualKeyCode::F4),
            toggle_ingame_ui: KeyMouse::Key(VirtualKeyCode::F6),
            attack: KeyMouse::Mouse(MouseButton::Left),
            second_attack: KeyMouse::Mouse(MouseButton::Right),
            roll: KeyMouse::Mouse(MouseButton::Middle),
        }
    }
}

/// `GameplaySettings` contains sensitivity and gameplay options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub crosshair_transp: f32,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            pan_sensitivity: 100,
            zoom_sensitivity: 100,
            crosshair_transp: 0.6,
        }
    }
}

/// `NetworkingSettings` stores server and networking settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkingSettings {
    pub username: String,
    pub servers: Vec<String>,
    pub default_server: usize,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "Username".to_string(),
            servers: vec!["server.veloren.net:38889".to_string()],
            default_server: 0,
        }
    }
}

/// `Log` stores the name to the log file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Log {
    pub file: PathBuf,
}

impl Default for Log {
    fn default() -> Self {
        Self {
            file: "voxygen.log".into(),
        }
    }
}

/// `GraphicsSettings` contains settings related to framerate and in-game visuals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub view_distance: u32,
    pub max_fps: u32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            view_distance: 5,
            max_fps: 60,
        }
    }
}

/// `AudioSettings` controls the volume of different audio subsystems and which
/// device is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,

    /// Audio Device that Voxygen will use to play audio.
    pub audio_device: Option<String>,
    pub audio_on: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.5,
            sfx_volume: 0.5,
            audio_device: None,
            audio_on: true,
        }
    }
}

/// `Settings` contains everything that can be configured in the settings.ron file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub controls: ControlSettings,
    pub gameplay: GameplaySettings,
    pub networking: NetworkingSettings,
    pub log: Log,
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub show_disclaimer: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            controls: ControlSettings::default(),
            gameplay: GameplaySettings::default(),
            networking: NetworkingSettings::default(),
            log: Log::default(),
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
            show_disclaimer: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Settings::get_settings_path();

        // If file doesn't exist, use the default settings.
        if let Ok(file) = fs::File::open(path) {
            ron::de::from_reader(file).expect("Error parsing settings")
        } else {
            Self::default()
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Settings::get_settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut config_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    fn get_settings_path() -> PathBuf {
        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");
        proj_dirs
            .config_dir()
            .join("settings")
            .with_extension("ron")
    }
}
