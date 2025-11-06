use anyhow::Result;
use clap::Parser;

use bourso_cli::settings::init_logger;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger()?;
    let cli = bourso_cli::cli::Cli::parse();
    bourso_cli::run(cli).await?;
    Ok(())
}
