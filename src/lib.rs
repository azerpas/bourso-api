use anyhow::{Context, Result};
use bourso_api::{
    account::{Account, AccountKind},
    client::{
        trade::{order::OrderSide, tick::QuoteTab},
        BoursoWebClient,
    },
    get_client,
};
use clap::ArgMatches;
use log::{info, warn};

mod settings;
use settings::{get_settings, save_settings, Settings};
mod validate;

#[cfg(not(tarpaulin_include))]
pub async fn parse_matches(matches: ArgMatches) -> Result<()> {
    use log::debug;

    let settings = match matches.get_one::<String>("credentials") {
        Some(credentials_path) => Settings::load(credentials_path)?,
        None => get_settings()?,
    };

    info!("Welcome to BoursoBank CLI 👋");
    info!(
        "ℹ️ - Version {}. Make sure you're running the latest version: {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY")
    );
    println!("");

    match matches.subcommand() {
        // These matches do not require authentication
        Some(("config", config_matches)) => {
            let customer_id = config_matches
                .get_one::<String>("username")
                .map(|s| s.as_str())
                .unwrap();
            save_settings(&Settings {
                customer_id: Some(customer_id.to_string()),
                password: None,
            })?;
            info!("Configuration saved ✅");
            return Ok(());
        }
        Some(("quote", quote_matches)) => {
            info!("Fetching quotes...");

            let symbol = quote_matches
                .get_one::<String>("symbol")
                .map(|s| s.as_str())
                .unwrap();
            let length = quote_matches
                .get_one::<String>("length")
                .map(|s| s.as_str())
                .unwrap();
            let interval = quote_matches
                .get_one::<String>("interval")
                .map(|s| s.as_str())
                .unwrap();
            let web_client: BoursoWebClient = get_client();

            let quotes = web_client
                .get_ticks(symbol, length.parse()?, interval.parse()?)
                .await?;

            match quote_matches.subcommand() {
                Some(("highest", _)) => {
                    info!("Highest quote: {:#?}", quotes.d.get_highest_value());
                }
                Some(("lowest", _)) => {
                    info!("Lowest quote: {:#?}", quotes.d.get_lowest_value());
                }
                Some(("volume", _)) => {
                    info!("Volume: {:#?}", quotes.d.get_volume());
                }
                Some(("average", _)) => {
                    info!("Average quote: {:#?}", quotes.d.get_average_value());
                }
                Some(("last", _)) => {
                    let quote: QuoteTab;

                    let last_quote = quotes.d.get_last_quote();
                    if last_quote.is_some() {
                        quote = last_quote.unwrap();
                    } else {
                        quote = quotes.d.quote_tab.last().unwrap().clone();
                    }

                    info!(
                        "Last quote: current: {}, open: {}, high: {}, low: {}, volume: {}",
                        quote.close, quote.open, quote.high, quote.low, quote.volume
                    );
                }
                _ => {
                    info!("Quotes:");
                    for quote in quotes.d.quote_tab.iter() {
                        info!(
                            "Quote day {}: Close: {}, Open: {}, High: {}, Low: {}, Volume: {}",
                            quote.date,
                            quote.close,
                            quote.open,
                            quote.high,
                            quote.low,
                            quote.volume
                        );
                    }
                }
            }

            return Ok(());
        }
        // These matches require authentication
        Some(("accounts", _))
        | Some(("transactions", _))
        | Some(("balance", _))
        | Some(("trade", _)) => (),
        _ => unreachable!(),
    }

    if settings.customer_id.is_none() {
        warn!("Please configure your customer id with `bourso config --username <customer_id>`");
        return Ok(());
    }
    let customer_id = settings.customer_id.unwrap();

    info!(
        "We'll try to log you in with your customer id: {}",
        customer_id
    );
    info!("If you want to change it, run `bourso config --username <customer_id>`");
    println!("");
    info!("We'll need your password to log you in. It will not be stored anywhere and will be asked everytime you run a command. The password will be hidden while typing.");

    // Get password from stdin
    let password = match settings.password {
        Some(password) => password,
        None => rpassword::prompt_password("Enter your password: ")
            .context("Failed to read password")?
            .trim()
            .to_string(),
    };

    let mut web_client: BoursoWebClient = get_client();
    web_client.init_session().await?;
    match web_client.login(&customer_id, &password).await {
        Ok(_) => {
            info!("Login successful ✅");
        }
        Err(e) => match e.downcast_ref() {
            Some(bourso_api::client::error::ClientError::MfaRequired) => {
                let mut mfa_required = true;
                let mut mfa_count = 0;
                while mfa_required {
                    // If MFA is passed twice, it means the user has passed an sms and email mfa
                    // which should clear the IP. We just need to reinitialize the session
                    // and login again to access the account.
                    if mfa_count == 2 {
                        warn!("MFA thresold reached. Trying to login again by reinitalizing the session.");
                        web_client = get_client();
                        web_client.init_session().await?;
                        match web_client.login(&customer_id, &password).await {
                            Ok(_) => {
                                info!("Login successful ✅");
                                break;
                            }
                            Err(e) => {
                                debug!("{:#?}", e);
                                return Err(e);
                            }
                        }
                    }
                    warn!("An MFA is required.");

                    let (otp_id, token, mfa_type) = web_client.request_mfa().await?;
                    let code = rpassword::prompt_password("Enter your MFA code: ")
                        .context("Failed to read MFA code")?
                        .trim()
                        .to_string();
                    match web_client.submit_mfa(mfa_type, otp_id, code, token).await {
                        Ok(_) => {
                            mfa_required = false;
                        }
                        Err(e) => match e.downcast_ref() {
                            Some(bourso_api::client::error::ClientError::MfaRequired) => {
                                mfa_count += 1;
                            }
                            _ => {
                                debug!("{:#?}", e);
                                return Err(e);
                            }
                        },
                    }
                }

                info!("MFA successful ✅");
            }
            _ => {
                debug!("{:#?}", e);
                return Err(e);
            }
        },
    }

    let accounts: Vec<Account>;

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
            accounts = web_client.get_accounts(Some(AccountKind::Trading)).await?;

            match trade_matches.subcommand() {
                Some(("order", order_matches)) => {
                    match order_matches.subcommand() {
                        Some(("new", new_order_matches)) => {
                            let account_id = new_order_matches
                                .get_one::<String>("account")
                                .map(|s| s.as_str())
                                .unwrap();

                            // Get account from previously fetched accounts
                            let account = accounts
                                .iter()
                                .find(|a| a.id == account_id)
                                .context("Account not found. Are you sure you have access to it? Run `bourso accounts` to list your accounts")?;

                            let side = new_order_matches.get_one::<OrderSide>("side").unwrap();
                            let quantity = new_order_matches.get_one::<usize>("quantity").unwrap();
                            let symbol = new_order_matches
                                .get_one::<String>("symbol")
                                .map(|s| s.as_str())
                                .unwrap();

                            let _ = web_client
                                .order(side.to_owned(), account, symbol, quantity.to_owned(), None)
                                .await?;
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
