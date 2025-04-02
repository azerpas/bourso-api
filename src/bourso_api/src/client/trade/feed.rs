use crate::client::{config::Config, BoursoWebClient};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

impl BoursoWebClient {
    #[cfg(not(tarpaulin_include))]
    pub async fn is_market_open(&self, symbol: &str) -> Result<bool> {
        let quote = match self.instrument_quote(symbol).await {
            Ok(quote) => quote,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to check if market is open: {}", e));
            }
        };

        let opening_time = quote.opening_time;
        let closing_time = quote.closing_time;

        let current_time = chrono::Local::now().time();

        let opening_time = chrono::NaiveTime::parse_from_str(&opening_time, "%H:%M:%S")?;
        let closing_time = chrono::NaiveTime::parse_from_str(&closing_time, "%H:%M:%S")?;

        Ok(current_time >= opening_time && current_time < closing_time)
    }

    #[cfg(not(tarpaulin_include))]
    pub async fn instrument_quote(&self, symbol: &str) -> Result<InstrumentQuoteResponse> {
        use anyhow::Context;

        let url = get_instrument_quote_url(&self.config, symbol)?;
        let response = self.client.get(url).send().await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!(
                "Failed to get instrument quote response: {}",
                response
            ));
        }

        let response: InstrumentQuoteResponse =
            serde_json::from_str(&response).context(format!(
                "Failed to parse instrument quote response. Response: {}",
                response
            ))?;

        Ok(response)
    }
}

fn get_feed_base_url(config: &Config) -> Result<String> {
    Ok(format!("{}/_public_/feed", config.api_url))
}

fn get_instrument_quote_url(config: &Config, symbol: &str) -> Result<String> {
    Ok(format!(
        "{}/instrument/quote/{}?_host=tradingboard.boursobank.com",
        get_feed_base_url(config)?,
        symbol
    ))
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentQuoteResponse {
    pub symbol: String,
    pub label: String,
    pub isin: String,
    pub last: f64,
    pub currency: String,
    pub previous_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub total_volume: i64,
    pub exchange_id: i64,
    pub exchange_code: String,
    pub exchange_label: String,
    pub opening_time: String,
    pub closing_time: String,
}
