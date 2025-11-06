use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{info, warn};

use crate::cli::{OrderArgs, OrderNewArgs, OrderSubcommands};
use crate::services::AuthService;
use crate::settings::FileSettingsStore;
use bourso_api::account::AccountKind;
use bourso_api::client::trade::order::OrderSide;

pub async fn handle(args: OrderArgs) -> Result<()> {
    match args.command {
        OrderSubcommands::New(n) => new_order(n).await,
        OrderSubcommands::List(_) => {
            warn!("Listing orders is coming soon.");
            Ok(())
        }
        OrderSubcommands::Cancel(_) => {
            warn!("Cancel order is coming soon.");
            Ok(())
        }
    }
}

async fn new_order(args: OrderNewArgs) -> Result<()> {
    let store = Arc::new(FileSettingsStore::new()?);
    let auth = AuthService::with_defaults(store);

    let Some(client) = auth.login().await? else {
        return Ok(());
    };

    // Choose a trading account and place the order
    let accounts = client.get_accounts(Some(AccountKind::Trading)).await?;
    let account = accounts
        .iter()
        .find(|a| a.id == args.account)
        .context("Account not found. Are you sure you have access to it? Run `bourso accounts` to list your accounts")?;

    let side: OrderSide = args.side;
    let quantity: usize = args.quantity as usize;
    let symbol = args.symbol;

    let _ = client.order(side, account, &symbol, quantity, None).await?;

    info!("Order submitted ✅");
    Ok(())
}
