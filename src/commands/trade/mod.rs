use anyhow::Result;

use crate::{
    cli::{TradeArgs, TradeCommands},
    AppCtx,
};

pub mod order;

pub async fn handle(args: TradeArgs, ctx: &AppCtx) -> Result<()> {
    match args.command {
        TradeCommands::Order(o) => order::handle(o, ctx).await,
    }
}
