use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::{
    cli::{OrderArgs, OrderNewArgs, OrderSubcommands},
    services::AuthService,
    AppCtx,
};

use bourso_api::account::AccountKind;
use bourso_api::types::OrderSide;

pub async fn handle(args: OrderArgs, ctx: &AppCtx) -> Result<()> {
    match args.command {
        OrderSubcommands::New(n) => new_order(n, ctx).await,
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

async fn new_order(args: OrderNewArgs, ctx: &AppCtx) -> Result<()> {
    let auth = AuthService::with_defaults(ctx.settings_store.as_ref());

    let Some(client) = auth.login().await? else {
        return Ok(());
    };

    // Choose a trading account and place the order
    let accounts = client.get_accounts(Some(AccountKind::Trading)).await?;
    let account = accounts
        .iter()
        .find(|a| a.id == args.account.as_ref().as_str())  // TODO: compare AccountId instead of String
        .context("Account not found. Are you sure you have access to it? Run `bourso accounts` to list your accounts")?;

    let side: OrderSide = args.side;
    let quantity: usize = args.quantity.get() as usize;
    let symbol = args.symbol;

    let _ = client
        .order(side, account, symbol.as_ref(), quantity, None)
        .await?;

    info!("Order submitted ✅");
    Ok(())
}
