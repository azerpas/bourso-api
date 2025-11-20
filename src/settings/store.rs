use anyhow::{anyhow, Context, Result};
use bourso_api::types::{ClientNumber, Password};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::{
    fs::{create_dir_all, read_to_string, write},
    io::ErrorKind,
    path::PathBuf,
};

use crate::settings::constants::{APP_NAME, APP_ORGANIZATION, APP_QUALIFIER, SETTINGS_FILE};

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(rename = "clientNumber")]
    pub client_number: Option<ClientNumber>,
    #[serde(rename = "password")]
    pub password: Option<Password>,
}

pub trait SettingsStore {
    fn load(&self) -> Result<Settings>;
    fn save(&self, settings: &Settings) -> Result<()>;
}

pub struct FsSettingsStore {
    path: PathBuf,
    create_if_missing: bool,
}

impl FsSettingsStore {
    /// Default location (XDG / platform config dir + SETTINGS_FILE)
    pub fn from_default_config_dir() -> Result<Self> {
        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
            .ok_or_else(|| anyhow!("Could not determine project directories"))?;

        Ok(Self {
            path: project_dirs.config_dir().join(SETTINGS_FILE),
            create_if_missing: true,
        })
    }

    /// Arbitrary path (e.g. provided via CLI)
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path,
            create_if_missing: false,
        }
    }

    fn ensure_directory(&self) -> Result<()> {
        if let Some(directory) = self.path.parent() {
            create_dir_all(directory).context("Failed to create settings directory")?;
        }
        Ok(())
    }
}

impl SettingsStore for FsSettingsStore {
    fn load(&self) -> Result<Settings> {
        self.ensure_directory()?;

        match read_to_string(&self.path) {
            Ok(content) => from_str(&content).context("Failed to deserialize settings"),

            Err(e) if self.create_if_missing && e.kind() == ErrorKind::NotFound => {
                // Only for "default config" mode AND only if the file is missing
                let defaults = Settings::default();
                self.save(&defaults)?;
                Ok(defaults)
            }

            Err(e) => Err(e).context("Failed to read settings file"),
        }
    }

    fn save(&self, settings: &Settings) -> Result<()> {
        self.ensure_directory()?;

        write(&self.path, to_string_pretty(settings)?).context("Failed to persist settings file")
    }
}
