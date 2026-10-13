use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GameStats {
    pub off: Option<u32>,
    pub on: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub start_with_windows: bool,
    pub auto_boost: bool,
    pub close_to_tray: bool,
    pub boost: bool,
    pub tweaks: HashMap<String, bool>,
    pub profiles: HashMap<String, bool>,
    pub selected_game: String,
    pub ultimate_guid: String,
    pub stats: HashMap<String, GameStats>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_with_windows: true,
            auto_boost: true,
            close_to_tray: true,
            boost: false,
            tweaks: crate::tweaks::recommended(),
            profiles: HashMap::new(),
            selected_game: "fortnite".into(),
            ultimate_guid: String::new(),
            stats: HashMap::new(),
        }
    }
}

fn path() -> std::path::PathBuf {
    crate::util::data_dir().join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(path(), json);
    }
}
