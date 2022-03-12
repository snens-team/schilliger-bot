use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_writer_pretty};
use std::env::current_dir;
use std::fs::{read_to_string, File};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub enum SettingsError {
    FailedToRead,
    InvalidSettings,
    FailedToCreateFile,
    FailedToWriteFile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub token: String,
    pub day_channel_id: u64,
    pub presence_channel_id: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            token: "Your Token".to_string(),
            day_channel_id: 0,
            presence_channel_id: 0,
        }
    }
}

/// Loads the settings of the discord bot
pub fn load_settings() -> Result<Settings, SettingsError> {
    let path = settings_path();

    let file = File::open(path).map_err(|_| SettingsError::FailedToCreateFile)?;
    serde_json::from_reader(file).map_err(|_| SettingsError::FailedToRead)
}

/// Saves the settings to the settings path
pub fn save_settings(settings: &Settings) -> Result<(), SettingsError> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap()
    }
    let file = File::create(&path).map_err(|_| SettingsError::FailedToCreateFile)?;
    to_writer_pretty(file, &settings).map_err(|_| SettingsError::FailedToWriteFile)?;
    Ok(())
}

/// Returns the path of the settings json file
fn settings_path() -> PathBuf {
    let mut path = current_dir().unwrap();
    path.push("settings.json");
    path
}
