use anyhow::{Result, Context};
use clap::{Arg, Command};

mod bourso;
mod settings;

use settings::{Settings, get_settings, save_settings};

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
        Some(("config", config_matches)) => {
            let customer_id = config_matches.get_one::<String>("username").map(|s| s.as_str()).unwrap();
            save_settings(&Settings { customer_id: Some(customer_id.to_string()) })?;
            println!("Configuration saved");
            return Ok(());
        }
        _ => unreachable!(),
    }

    let settings = get_settings()?;

    if settings.customer_id.is_none() {
        println!("Please configure your customer id with `bourso config --username <customer_id>`");
        return Ok(());
    }
    let customer_id = settings.customer_id.unwrap();

    // Get password from stdin
    let password = rpassword::prompt_password("Your password: ")
        .context("Failed to read password")?
        .trim()
        .to_string();

    let mut web_client: bourso::client::BoursoWebClient = bourso::get_client();
    web_client.init_session().await?;
    web_client.login(&customer_id, &password).await?;

    Ok(())
}


