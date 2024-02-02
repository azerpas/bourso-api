use anyhow::{Result, Context};
use bourso_api::{client::{BoursoWebClient, trade::order::OrderSide}, get_client, account::{Account, AccountKind}};
use clap::ArgMatches;
use log::{info, warn};

mod settings;
use settings::{Settings, get_settings, save_settings};
mod validate;

pub async fn parse_matches(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        // These matches require authentication
        Some(("accounts", _)) | Some(("transactions", _)) | Some(("balance", _)) | Some(("trade", _)) => {
            ()
        }
        Some(("config", config_matches)) => {
            let customer_id = config_matches.get_one::<String>("username").map(|s| s.as_str()).unwrap();
            save_settings(&Settings { customer_id: Some(customer_id.to_string()) })?;
            info!("Configuration saved ✅");
            return Ok(());
        }
        _ => unreachable!(),
    }

    let settings = get_settings()?;

    if settings.customer_id.is_none() {
        warn!("Please configure your customer id with `bourso config --username <customer_id>`");
        return Ok(());
    }
    let customer_id = settings.customer_id.unwrap();

    info!("Welcome to BoursoBank CLI 👋");
    info!("ℹ️ Version {}. Make sure you're running the latest version: {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_REPOSITORY"));
    println!("");
    info!("We'll try to log you in with your customer id: {}", customer_id);
    info!("If you want to change it, run `bourso config --username <customer_id>`");
    println!("");
    info!("We'll need your password to log you in. It will not be stored anywhere and will be asked everytime you run a command. The password will be hidden while typing.");

    // Get password from stdin
    let password = rpassword::prompt_password("Enter your password: ")
        .context("Failed to read password")?
        .trim()
        .to_string();

    let mut web_client: BoursoWebClient = get_client();
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

            info!("Found {} accounts", accounts.len());
            println!("{:#?}", accounts);
        }

        Some(("trade", trade_matches)) => {
            accounts = web_client
                .get_accounts(Some(AccountKind::Trading)).await?;

            match trade_matches.subcommand() {
                Some(("order", order_matches)) => {

                    match order_matches.subcommand() {
                        Some(("new", new_order_matches)) => {
                            let account_id = new_order_matches
                                .get_one::<String>("account").map(|s| s.as_str()).unwrap();
                            
                            // Get account from previously fetched accounts
                            let account = accounts
                                .iter()
                                .find(|a| a.id == account_id)
                                .context("Account not found. Are you sure you have access to it? Run `bourso accounts` to list your accounts")?;

                            let side = new_order_matches.get_one::<OrderSide>("side").unwrap();
                            let quantity = new_order_matches.get_one::<usize>("quantity").unwrap();
                            let symbol = new_order_matches.get_one::<String>("symbol").map(|s| s.as_str()).unwrap();

                            let _ = web_client
                                .order(
                                    side.to_owned(), 
                                    account, 
                                    symbol, 
                                    quantity.to_owned(), 
                                    None
                                ).await?;
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}