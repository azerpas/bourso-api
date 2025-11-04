use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::{fs, path::PathBuf};

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
    directory: PathBuf, // injected base directory (e.g., platform config directory)
    file: &'static str, // e.g., "settings.json"
}

impl FileSettingsStore {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            file: "settings.json",
        }
    }

    /// Convenience: build from ProjectDirs config directory.
    /// `qualifier` can be "" if you don’t have one. Example:
    ///   - Windows:   %APPDATA%\<qualifier>\<org>\<app>\settings.json
    ///   - macOS:     ~/Library/Application Support/<app>/settings.json
    ///   - Linux:     ~/.config/<app>/settings.json
    pub fn from_project_dirs(
        qualifier: &str,
        organization: &str,
        application: &str,
    ) -> Result<Self> {
        let project_directories = ProjectDirs::from(qualifier, organization, application)
            .ok_or_else(|| anyhow!("Could not determine project directories"))?;
        Ok(Self::new(project_directories.config_dir().to_path_buf()))
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
        fs::write(&self.path(), to_string_pretty(settings)?)
            .with_context(|| format!("Failed to persist settings file: {}", self.path().display()))
    }
}
