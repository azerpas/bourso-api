use std::fs;
use std::io::prelude::*;
use anyhow::{Result, Context};
use directories::UserDirs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub customer_id: Option<String>,
}

pub fn get_settings() -> Result<Settings> {
    let user_dirs = UserDirs::new().context("Failed to get user directories")?;
    let mut path = user_dirs.home_dir().to_path_buf();
    path = path.join(".bourso/settings.json");
    let file_content = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(_) => {
            // Create the settings file if it doesn't exist
            save_settings(&Settings { customer_id: None })?;
            return Ok(Settings { customer_id: None });
        },
    };

    let settings: Settings = serde_json::from_str(&file_content)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize settings: {}\nPlease make sure the settings file is valid.", e))?;
    Ok(settings)
}

/// Save the settings to the settings file, if it doesn't exist, create it
pub fn save_settings(settings: &Settings) -> Result<()> {
    let user_dirs = UserDirs::new().context("Failed to get user directories")?;
    let mut path = user_dirs.home_dir().to_path_buf();
    // Create the .bourso directory if it doesn't exist
    path = path.join(".bourso");
    fs::create_dir_all(&path)?;
    path = path.join("settings.json");
    let mut file = fs::File::create(&path).context("Failed to create settings file")?;
    let json = serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;
    file.write_all(json.as_bytes()).context("Failed to write settings file")?;
    Ok(())
}