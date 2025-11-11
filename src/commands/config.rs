use anyhow::Result;
use tracing::info;

use crate::{cli::ConfigArgs, settings::Settings, AppCtx};

pub async fn handle(args: ConfigArgs, ctx: &AppCtx) -> Result<()> {
    ctx.settings_store.save(&Settings {
        client_number: Some(args.client_number),
        password: None,
    })?;
    info!("Configuration saved successfully ✅");
    Ok(())
}
