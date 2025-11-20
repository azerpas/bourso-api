use anyhow::Result;
use clap::Parser;

use bourso_cli::cli::Cli;
use bourso_cli::{init_logger, run};

#[tokio::main]
async fn main() -> Result<()> {
    init_logger()?;
    let cli = Cli::parse();
    run(cli).await?;
    Ok(())
}
