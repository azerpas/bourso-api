use anyhow::Result;

pub mod cli;
pub mod commands;
pub mod services;
pub mod settings;
pub mod ux;

pub use services::AuthService;
pub use settings::init_logger;
pub use settings::{FileSettingsStore, JsonFileSettingsStore, Settings, SettingsStore};
pub use ux::TextProgressBar;

pub struct AppCtx {
    pub settings_store: Box<dyn SettingsStore>,
}

pub async fn run(cli: cli::Cli) -> Result<()> {
    let settings_store: Box<dyn SettingsStore> = match cli.credentials.clone() {
        Some(path) => Box::new(JsonFileSettingsStore::new(path)),
        None => Box::new(FileSettingsStore::new()?),
    };
    let ctx = AppCtx { settings_store };

    use cli::Commands::*;
    match cli.command {
        Config(args) => commands::config::handle(args, &ctx).await,
        Accounts(args) => commands::accounts::handle(args, &ctx).await,
        Trade(args) => commands::trade::handle(args, &ctx).await,
        Quote(args) => commands::quote::handle(args).await,
        Transfer(args) => commands::transfer::handle(args, &ctx).await,
    }
}
