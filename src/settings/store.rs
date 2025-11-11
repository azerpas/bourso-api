use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::{fs, path::PathBuf};

use crate::settings::consts::{APP_NAME, APP_ORGANIZATION, APP_QUALIFIER, SETTINGS_FILE};

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(rename = "clientNumber")]
    pub client_number: Option<String>,
    #[serde(rename = "password")]
    pub password: Option<String>,
}

pub trait SettingsStore {
    fn load(&self) -> Result<Settings>;
    fn save(&self, settings: &Settings) -> Result<()>;
}

pub struct FileSettingsStore {
    directory: PathBuf, // platform config directory (from ProjectDirs)
    file: &'static str, // "settings.json"
}

impl FileSettingsStore {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
            .ok_or_else(|| anyhow!("Could not determine project directories"))?;

        Ok(Self {
            directory: project_dirs.config_dir().to_path_buf(),
            file: SETTINGS_FILE,
        })
    }

    fn path(&self) -> PathBuf {
        self.directory.join(self.file)
    }
}

impl SettingsStore for FileSettingsStore {
    fn load(&self) -> Result<Settings> {
        fs::create_dir_all(&self.directory).with_context(|| {
            format!(
                "Failed to create settings directory: {}",
                self.directory.display()
            )
        })?;
        let path = self.path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                let defaults = Settings::default();
                self.save(&defaults)?;
                return Ok(defaults);
            }
        };
        from_str(&content).context("Failed to deserialize settings")
    }

    fn save(&self, settings: &Settings) -> Result<()> {
        fs::create_dir_all(&self.directory).with_context(|| {
            format!(
                "Failed to create settings directory: {}",
                self.directory.display()
            )
        })?;
        fs::write(self.path(), to_string_pretty(settings)?)
            .with_context(|| format!("Failed to persist settings file: {}", self.path().display()))
    }
}

pub struct JsonFileSettingsStore {
    path: PathBuf,
}

impl JsonFileSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

impl SettingsStore for JsonFileSettingsStore {
    fn load(&self) -> Result<Settings> {
        let content = fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read settings file: {}", self.path.display()))?;
        from_str(&content).context("Failed to deserialize settings")
    }

    fn save(&self, settings: &Settings) -> Result<()> {
        fs::write(self.path(), to_string_pretty(settings)?)
            .with_context(|| format!("Failed to persist settings file: {}", self.path().display()))
    }
}
