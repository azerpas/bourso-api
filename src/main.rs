use anyhow::Result;
use clap::CommandFactory;

#[tokio::main]
async fn main() -> Result<()> {
    bourso_cli::settings::init_logger()?;

    let matches = bourso_cli::cli::Cli::command().get_matches();

    bourso_cli::parse_matches(matches).await
}
