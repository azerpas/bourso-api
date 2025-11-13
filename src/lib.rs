use anyhow::Result;

pub mod cli;
pub mod commands;
pub mod services;
pub mod settings;
pub mod ux;

pub use services::AuthService;
pub use settings::{init_logger, FsSettingsStore, Settings, SettingsStore};
pub use ux::TextProgressBar;

pub struct AppCtx {
    pub settings_store: Box<dyn SettingsStore>,
}

pub async fn run(cli: cli::Cli) -> Result<()> {
    let cli::Cli {
        credentials,
        command,
    } = cli;

    let settings_store: Box<dyn SettingsStore> = match credentials {
        Some(path) => Box::new(FsSettingsStore::from_path(path)),
        None => Box::new(FsSettingsStore::from_default_config_dir()?),
    };
    let ctx = AppCtx { settings_store };

    match command {
        cli::Commands::Config(args) => commands::config::handle(args, &ctx).await,
        cli::Commands::Accounts(args) => commands::accounts::handle(args, &ctx).await,
        cli::Commands::Trade(args) => commands::trade::handle(args, &ctx).await,
        cli::Commands::Quote(args) => commands::quote::handle(args).await,
        cli::Commands::Transfer(args) => commands::transfer::handle(args, &ctx).await,
    }
}
