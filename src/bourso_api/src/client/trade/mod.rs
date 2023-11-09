pub mod error;
pub mod order;

use anyhow::Result;

use crate::account::{Account, AccountKind};

use super::{
    BoursoWebClient,
    config::Config,
};

impl BoursoWebClient {
    pub async fn trade() {}
}

fn get_trading_base_url(config: &Config) -> Result<String> {
    if config.user_hash.is_none() {
        return Err(anyhow::anyhow!("User hash is not set"));
    }

    Ok(
        format!(
            "{}/_user_/_{}_/trading",
            config.api_url,
            config.user_hash.as_ref().unwrap()
        )
    )
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

fn get_trading_is_first_order_url(config: &Config) -> Result<String> {
    Ok(
        format!(
            "{}/order/isfirstorder?_host=tradingboard.boursobank.com",
            get_trading_base_url(config)?
        )
    )
}