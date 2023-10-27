use anyhow::{Result, Context};
use clap::{Arg, Command};

mod bourso;
mod settings;

use settings::{Settings, get_settings, save_settings};
use bourso::{
    account::{
        Account, 
        AccountKind
    },
    client::BoursoWebClient 
};

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

    match matches.subcommand() {
        // These matches require authentication
        Some(("accounts", _)) | Some(("transactions", _)) | Some(("balance", _)) => {
            println!("transactions");
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

    let mut web_client: BoursoWebClient = bourso::get_client();
    web_client.init_session().await?;
    web_client.login(&customer_id, &password).await?;

    let mut accounts: Vec<Account> = vec![];

    match matches.subcommand() {
        Some(("accounts", sub_matches)) => {
            if sub_matches.contains_id("bank") {
                accounts = web_client.get_accounts(Some(AccountKind::Banking)).await?;
            } else if sub_matches.contains_id("saving") {
                accounts = web_client.get_accounts(Some(AccountKind::Savings)).await?;
            } else if sub_matches.contains_id("trading") {
                accounts = web_client.get_accounts(Some(AccountKind::Trading)).await?;
            } else if sub_matches.contains_id("loans") {
                accounts = web_client.get_accounts(Some(AccountKind::Loans)).await?;
            } else {
                accounts = web_client.get_accounts(None).await?;
            }
        }
        _ => unreachable!(),
    }

    println!("{:#?}", accounts);

    Ok(())
}


