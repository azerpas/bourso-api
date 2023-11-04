use anyhow::Result;
use clap::{Arg, Command};

mod settings;

#[tokio::main]
async fn main() -> Result<()> {
    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

    let matches = Command::new("bourso")
        .version(VERSION.unwrap_or("0.0.1"))
        .author("@azerpas")
        .about("BoursoBank/Boursorama CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("accounts")
                .about("Manage your saving accounts")
                .arg(
                    Arg::new("bank")
                        .long("banking")
                        .help("List all your base banking accounts")
                )
                .arg(
                    Arg::new("saving")
                        .long("saving")
                        .help("List all your saving accounts")
                )
                .arg(
                    Arg::new("trading")
                        .long("trading")
                        .help("List all your trading accounts")
                )
                .arg(
                    Arg::new("loans")
                        .long("loans")
                        .help("List all your loans")
                )
        )
        .subcommand(
            Command::new("config")
                .about("Configure BoursoBank/Boursorama CLI")
                .arg(
                    Arg::new("username")
                        .short('u')
                        .long("username")
                        .help("Your customer id")
                        .required(true)
                )
        )
        // .subcommand( // interactive mode
        .subcommand(
            Command::new("trade")
                .about("Trade on your trading accounts")
                .subcommand(
                    Command::new("list")
                        .about("List all your current orders")
                )
                .subcommand(
                    Command::new("buy")
                        .about("Buy a stock")
                )
                .arg(
                    Arg::new("account")
                        .short('a')
                        .long("account")
                        .help(
                            r#"The account to use by its 'name' (e.g: 'PEA') or 'id' (e.g: 'e51f635524a7d506e4d4a7a8088b6278').
You can get these infos with the command `bourso accounts`"#
                        )
                        .default_value("PEA")
                )
        )
        .get_matches();

    bourso_cli::parse_matches(matches).await?;

    Ok(())
}


