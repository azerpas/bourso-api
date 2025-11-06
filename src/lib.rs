use anyhow::Result;

pub mod cli;
pub mod commands;
pub mod services;
pub mod settings;

pub use services::AuthService;
pub use settings::init_logger;
pub use settings::{FileSettingsStore, Settings, SettingsStore};

pub async fn run(cli: cli::Cli) -> Result<()> {
    use cli::Commands::*;
    match cli.command {
        Config(args) => commands::config::handle(args).await,
        Accounts(args) => commands::accounts::handle(args).await,
        Trade(args) => commands::trade::handle(args).await,
        Quote(args) => commands::quote::handle(args).await,
        Transfer(args) => commands::transfer::handle(args).await,
    }
}
