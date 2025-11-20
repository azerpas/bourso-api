use anyhow::Result;
use tracing::info;

use crate::cli::{QuoteArgs, QuoteView};

pub async fn handle(args: QuoteArgs) -> Result<()> {
    info!("Fetching quotes ...");

    let client = bourso_api::get_client();
    let quotes = client
        .get_ticks(
            args.symbol.as_ref(),
            args.length.days(),
            args.period.value(),
        )
        .await?;

    match args.view {
        Some(QuoteView::Highest) => {
            let highest = quotes.d.get_highest_value();
            info!(?highest, "Highest quote");
        }
        Some(QuoteView::Lowest) => {
            let lowest = quotes.d.get_lowest_value();
            info!(?lowest, "Lowest quote");
        }
        Some(QuoteView::Average) => {
            let average = quotes.d.get_average_value();
            info!(?average, "Average quote");
        }
        Some(QuoteView::Volume) => {
            let volume = quotes.d.get_volume();
            info!(?volume, "Volume");
        }
        Some(QuoteView::Last) => {
            let last = quotes.d.get_last_quote();
            info!(?last, "Last quote");
        }
        None => {
            info!("No view specified, displaying all quotes");
            for quote in quotes.d.get_quotes() {
                info!(?quote, "Quote");
            }
        }
    }

    Ok(())
}
