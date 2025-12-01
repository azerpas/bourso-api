pub mod error;
pub mod feed;
pub mod order;
pub mod tick;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::account::{Account, AccountKind};

use super::{config::Config, BoursoWebClient};

impl BoursoWebClient {
    pub async fn trade() {}
    pub async fn get_trading_summary(&self, account: Account) -> Result<Vec<TradingSummaryItem>> {
        let url = get_trading_summary_url(&self.config, account)?;
        let response = self.client.get(url).send().await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!(
                "Failed to get trading summary response: {}",
                response
            ));
        }

        let summary: Vec<TradingSummaryItem> = serde_json::from_str(&response).context(format!(
            "Failed to parse trading summary response. Response: {}",
            response
        ))?;
        Ok(summary)
    }
}

fn get_trading_base_url(config: &Config) -> Result<String> {
    if config.user_hash.is_none() {
        return Err(anyhow::anyhow!("User hash is not set"));
    }

    Ok(format!(
        "{}/_user_/_{}_/trading",
        config.api_url,
        config.user_hash.as_ref().unwrap()
    ))
}

fn get_trading_summary_url(config: &Config, account: Account) -> Result<String> {
    if account.kind != AccountKind::Trading {
        return Err(anyhow::anyhow!("Account is not a trading account"));
    }

    Ok(
        format!(
            "{}/accounts/summary/{}?_host=tradingboard.boursobank.com&position=ACCOUNTING&responseFormat=true",
            get_trading_base_url(config)?,
            account.id
        )
    )
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingSummaryItem {
    /// Either "account" or "positions"
    pub id: String,
    /// Will be Some if TradingSummaryItem is of id "account"
    pub account: Option<AccountSummary>,
    /// Will be Some if TradingSummaryItem is of id "positions"
    pub positions: Option<Vec<PositionSummary>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSummary {
    /// The symbol of the position ie. 1rTCW8
    pub symbol: String,
    /// The name of the position ie. AMUNDI ETF MSCI WORLD UCITS ETF
    pub label: String,
    pub permalink: String,
    pub quantity: SummaryValue,
    pub buying_price: SummaryValue,
    pub amount: SummaryValue,
    pub last: SummaryValue,
    /// Variation ?
    pub var: SummaryValue,
    pub gain_loss: SummaryValue,
    pub gain_loss_percent: SummaryValue,
    /// YYYY-MM-DD
    pub last_movement_date: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryValue {
    pub value: f64,
    pub decimals: u64,
    pub currency: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    /// Name of the account
    pub name: String,
    pub currency: String,
    /// eg "TRADING"
    pub type_category: String,
    /// YYYY-MM-DD
    pub activation_date: String,
    pub balance: SummaryValue,
    pub cash: SummaryValue,
    pub valuation: SummaryValue,
    pub total: SummaryValue,
    pub gain_loss: SummaryValue,
    pub gain_loss_percent: SummaryValue,
    pub liquidation_amount: SummaryValue,
    /// Cash deposited
    pub contribution: i64,
}
