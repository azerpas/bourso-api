use anyhow::Result;

use crate::cli::{TradeArgs, TradeCommands};

pub mod order;

pub async fn handle(args: TradeArgs) -> Result<()> {
    match args.command {
        TradeCommands::Order(o) => order::handle(o).await,
    }
}
