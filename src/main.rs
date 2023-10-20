use anyhow::{Result, Context};
use clap::{Arg, Command};

mod bourso;

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

        )
        .args([
            Arg::new("username")
                .short('u')
                .long("username")
                .help("Your Boursorama username")
                .required(true),
        ])
        .get_matches();

    match matches.subcommand() {
        Some(("accounts", _)) => {
            println!("accounts");
        }
        Some(("transactions", _)) => {
            println!("transactions");
        }
        Some(("balance", _)) => {
            println!("login");
        }
        _ => unreachable!(),
    }

    Ok(())
}


