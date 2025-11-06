use anyhow::Result;
use tracing::info;

use crate::cli::ConfigArgs;
use crate::settings::{FileSettingsStore, Settings, SettingsStore};

pub async fn handle(args: ConfigArgs) -> Result<()> {
    FileSettingsStore::new()?.save(&Settings {
        client_number: Some(args.client_number),
        password: None,
    })?;
    info!("Configuration saved successfully ✅");
    Ok(())
}
