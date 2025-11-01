use anyhow::{Context, Result};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    #[serde(rename = "clientId")]
    pub customer_id: Option<String>,
    #[serde(rename = "password")]
    pub password: Option<String>,
}

#[cfg(not(tarpaulin_include))]
impl Settings {
    pub fn load(path: &str) -> Result<Settings> {
        let file_content = match fs::read_to_string(path) {
            Ok(data) => data,
            Err(_) => {
                return Err(anyhow::anyhow!("Failed to read settings file"));
            }
        };

        let settings: Settings = serde_json::from_str(&file_content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to deserialize settings: {}\nPlease make sure the settings file is valid.",
                e
            )
        })?;

        Ok(settings)
    }
}

#[cfg(not(tarpaulin_include))]
pub fn get_settings() -> Result<Settings> {
    let user_dirs = UserDirs::new().context("Failed to get user directories")?;
    let mut path = user_dirs.home_dir().to_path_buf();
    path = path.join(".bourso/settings.json");
    let file_content = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(_) => {
            // Create the settings file if it doesn't exist
            save_settings(&Settings {
                customer_id: None,
                password: None,
            })?;
            return Ok(Settings {
                customer_id: None,
                password: None,
            });
        }
    };

    let settings: Settings = serde_json::from_str(&file_content).map_err(|e| {
        anyhow::anyhow!(
            "Failed to deserialize settings: {}\nPlease make sure the settings file is valid.",
            e
        )
    })?;
    Ok(settings)
}

/// Save the settings to the settings file, if it doesn't exist, create it
#[cfg(not(tarpaulin_include))]
pub fn save_settings(settings: &Settings) -> Result<()> {
    let user_dirs = UserDirs::new().context("Failed to get user directories")?;
    let mut path = user_dirs.home_dir().to_path_buf();
    // Create the .bourso directory if it doesn't exist
    path = path.join(".bourso");
    fs::create_dir_all(&path)?;
    path = path.join("settings.json");
    let mut file = fs::File::create(&path).context("Failed to create settings file")?;
    let json = serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;
    file.write_all(json.as_bytes())
        .context("Failed to write settings file")?;
    Ok(())
}

pub fn init_logger() -> Result<()> {
    use std::io::IsTerminal;
    use std::{fs, io};
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Create ~/.bourso/bourso.log if it doesn't exist
    let user_dirs = UserDirs::new().context("Failed to get user directories")?;
    let mut path = user_dirs.home_dir().to_path_buf();
    path.push(".bourso");
    fs::create_dir_all(&path)?;
    path.push("bourso.log");

    // Pretty console (stderr), filtered by RUST_LOG
    let console_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_ansi(IsTerminal::is_terminal(&io::stderr()))
        .with_level(true)
        .with_target(true)
        .without_time()
        .compact()
        .fmt_fields({
            fmt::format::debug_fn(move |writer, field, value| {
                if field.name() == "message" {
                    write!(writer, "{:?}", value)?;
                }
                Ok(())
            })
        })
        .with_filter(env_filter.clone());

    // JSON file (capture everything)
    let log_path = path.clone();
    let json_layer = fmt::layer()
        .with_writer(move || {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .expect("open ~/.bourso/bourso.log")
        })
        .json()
        .with_target(true)
        .with_level(true)
        .flatten_event(true)
        .with_filter(LevelFilter::TRACE);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(json_layer)
        .init();

    Ok(())
}
