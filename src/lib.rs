use anyhow::{Result, Context};
use bourso_api::{client::BoursoWebClient, get_client, account::{Account, AccountKind}};
use clap::ArgMatches;

mod settings;
use settings::{Settings, get_settings, save_settings};

pub async fn parse_matches(matches: ArgMatches) -> Result<()> {
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
        }
        _ => unreachable!(),
    }

    println!("{:#?}", accounts);

    Ok(())
}