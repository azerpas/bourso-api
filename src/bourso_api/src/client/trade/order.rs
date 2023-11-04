use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use serde_json::Value;

use crate::{
    account::{Account, AccountKind}, client::config::Config, 
};

use super::{BoursoWebClient, get_trading_base_url};

impl BoursoWebClient {
    /// Place an order
    /// 
    /// # Arguments
    /// 
    /// * `side` - Order side (buy or sell)
    /// * `account` - Account to use. Must be a trading account
    /// * `symbol` - Symbol to trade
    /// * `quantity` - Quantity to trade
    /// * `order_data` - Order data. If not set, will be fetched from Bourso API and filled with the given parameters
    pub async fn order(&self, side: OrderSide, account: &Account, symbol: &str, quantity: usize, order_data: Option<OrderData>) -> Result<()> {

        if account.kind != AccountKind::Trading {
            return Err(anyhow::anyhow!("Account is not a trading account"));
        }

        let response = self.prepare(account, symbol).await?;

        // Either the order data set by the user
        // or a prefilled data object fetched from Bourso API
        let mut order_data = match order_data {
            Some(data) => data,
            None => response.prefill_order_data.clone(),
        };

        let last_price = response.symbol.last_price;

        // As either the data received by Bourso API or the data given by the user can contain
        // and order quantity set to none, we forcefully define it here
        if order_data.order_quantity.is_none() {
            order_data.order_quantity = Some(quantity);
        }

        if order_data.order_price_limit.is_none() && order_data.order_type == OrderKind::Limit {
            if order_data.order_amount.is_some() {
                // Use quoted market price or given user price
                order_data.order_price_limit = order_data.order_amount;
            } else {
                // Use the last price fetched
                order_data.order_price_limit = Some(last_price);
            }
        } // else TODO: other types of orders data definition

        if order_data.order_side.is_none() {
            order_data.order_side = Some(side);
        }

        if order_data.order_expiration_date.is_none() {
            // Set expiration date to date given by the API
            order_data.order_expiration_date = response.prefill_order_data.order_validity;
        } else {
            // Set order_data.order_expiration_date to today
            order_data.order_expiration_date = Some(
                chrono::Utc::now()
                .format("%Y-%m-%d")
                .to_string()
            );
        }

        order_data.resource_id = response.prefill_order_data.resource_id;
        
        self.check(&order_data).await?;

        Ok(())
    }   

    /// Prepare an order
    /// 
    /// # Arguments
    /// 
    /// * `account` - Account to use. Must be a trading account
    /// * `symbol` - Symbol to trade
    /// 
    /// # Returns 
    /// 
    /// An order prepare response
    async fn prepare(&self, account: &Account, symbol: &str) -> Result<OrderPrepareResponse> {
        let url = get_order_prepare_url(&self.config, account, symbol)?;
        let response = self.client
            .get(url)
            .send()
            .await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!("Failed to get order prepare response: {}", response));
        }

        let response: OrderPrepareResponse = serde_json::from_str(&response)
            .context("Failed to parse order prepare response")?;
        
        Ok(response)
    }

    /// Check if an order is valid
    /// 
    /// # Arguments
    /// 
    /// * `data` - Order data to check
    /// 
    /// # Returns
    /// 
    /// An order check response
    async fn check(&self, data: &OrderData) -> Result<OrderCheckResponse> {
        let url = get_order_check_url(&self.config)?;
        let response = self.client
            .post(url)
            .body(serde_json::to_string(data)?)
            .send()
            .await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!("Failed to get order prepare response: {}", response));
        }

        let response: OrderCheckResponse = serde_json::from_str(&response)
            .context("Failed to parse order prepare response")?;
        
        Ok(response)
    }
}

fn get_order_url(config: &Config) -> Result<String> {
    let trading_url = get_trading_base_url(config)?;

    Ok(
        format!(
            "{}/order",
            trading_url
        )
    )
}

fn get_order_prepare_url(config: &Config, account: &Account, symbol: &str) -> Result<String> {
    Ok(
        format!(
            "{}/prepare?_host=tradingboard.boursobank.com&searchExtendedHours=false&selectedAccount={}&symbol={}",
            get_order_url(config)?,
            account.id,
            symbol
        )
    )
}

fn get_order_check_url(config: &Config) -> Result<String> {
    Ok(
        format!(
            "{}/ordersimple/check",
            get_trading_base_url(config)?
        )
    )
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPrepareResponse {
    pub resource_id: String,
    pub is_pcc: bool,
    pub pcc_rights: PccRights,
    pub has_right_to_assign: bool,
    pub has_right_to_force: bool,
    pub position: Position,
    pub account: PrepareOrderAccount,
    pub account_fiscality: AccountFiscality,
    pub account_fees_profile: String,
    pub pending_executed_orders: PendingExecutedOrders,
    pub acceptability_messages: Vec<Value>,
    pub symbol: Symbol,
    pub prepare_order_data: PrepareOrderData,
    pub prefill_order_data: OrderData,
    pub opcvm_message: String,
    pub dici_message: String,
    pub performance_url: String,
    pub execution_policy_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PccRights {
    pub allocate: bool,
    pub force: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub cash: f64,
    pub srd_coverage: f64,
    pub quantity: i64,
    pub srd_quantity: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareOrderAccount {
    pub has_pfm: bool,
    pub rib: String,
    pub iban: String,
    pub bic: String,
    pub account_number: String,
    pub name: String,
    pub balance: f64,
    pub internal: bool,
    pub currency: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub professional: bool,
    pub subtype: String,
    pub role: String,
    pub bank_id: String,
    pub bank_name: String,
    pub cash_out: i64,
    pub cash_in: i64,
    pub account_key: String,
    pub pfm_account_key: Value,
    pub type_category: String,
    pub has_unregular_operations: bool,
    pub short_name: String,
    pub minor: bool,
    pub contact_id_owner: Value,
    #[serde(rename = "isKADOR")]
    pub is_kador: bool,
    pub profile_type: Value,
    pub details: Details,
    pub has_incident: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub first_cash_transfer_date: String,
    pub gain_losses_percent: f64,
    pub done_gain_losses_percent: i64,
    pub cash: String,
    pub gain_losses: f64,
    pub done_gain_losses: i64,
    pub clearance_balance: i64,
    pub stocks: f64,
    pub date: String,
    pub next_liquidation_date: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountFiscality {
    #[serde(rename = "latGL")]
    pub lat_gl: f64,
    #[serde(rename = "realGL")]
    pub real_gl: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingExecutedOrders {
    pub pending: i64,
    pub executed: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub exchange_label: String,
    pub symbol: String,
    pub nb_decimals: i64,
    pub currency: String,
    pub label: String,
    pub isin: String,
    pub last_price: f64,
    pub fund_morning_star_pdf_url: String,
    pub direct_issuer_kid_url: Value,
    pub priips_kid_url: Value,
    pub allow_tactical_orders: bool,
    pub details: Details2,
    pub extended_hours: ExtendedHours,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Details2 {
    pub opcvm: bool,
    pub affiliated: bool,
    pub direct_issuer: bool,
    pub tracker: bool,
    pub turbo: bool,
    pub warrant: bool,
    pub euronext: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedHours {
    pub associated_symbol: String,
    pub lox_symbol: String,
    pub is_eligible: bool,
    pub lox_exchange_id: String,
    pub is_ost: bool,
    pub is_open: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareOrderData {
    pub min_expire_tm: String,
    pub max_expire_tm: String,
    pub invalid_dates_list: Vec<String>,
    pub list_ord_type: ListOrdType,
    pub list_risk_md: Vec<String>,
    pub side_list: Vec<String>,
    // pub config_ord_type: ConfigOrdType,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrdType {
    pub b: Vec<String>,
    pub s: Vec<String>,
}

/// Type of order
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
enum OrderKind {
    #[default]
    #[serde(rename = "LIM")]
    Limit,
    #[serde(rename = "ATP")]
    Market,
    /// Seuil de déclenchement
    #[serde(rename = "STP")]
    StopLoss,
    /// Plage de déclenchement
    #[serde(rename = "SLM")]
    StopLossMargin,
    #[serde(rename = "TSO")]
    TrailingStopOrder,
    /// One Cancels the Other order
    #[serde(rename = "OCO")]
    OneCancelsOther,
    #[serde(rename = "TAL")]
    TradeAtLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum OrderSide {
    #[default]
    #[serde(rename = "B")]
    Buy,
    #[serde(rename = "S")]
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OrderData {
    #[serde(rename = "orderType")]
    order_type: OrderKind,
    #[serde(rename = "orderSide")]
    order_side: Option<OrderSide>,
    #[serde(rename = "orderQuantity")]
    order_quantity: Option<usize>,
    /// Expiration date in format "2022-11-01"
    #[serde(rename = "orderExpirationDate")]
    order_expiration_date: Option<String>,
    #[serde(rename = "orderRiskMode")]
    order_risk_mode: String,
    /// To use at the `/ordersimple/check` endpoint
    #[serde(rename = "orderPriceLimit")]
    order_price_limit: Option<f64>,
    /// Received at the `/order/prepare` endpoint 
    #[serde(rename = "orderAmount")]
    order_amount: Option<f64>,
    #[serde(rename = "resourceId")]
    resource_id: Option<String>,
    /// Received at the `/order/prepare` endpoint
    /// Validity date in format "2022-11-01"
    #[serde(rename = "orderValidity")]
    order_validity: Option<String>,
    
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "buyingPower")]
    pub buying_power: Option<f64>,
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "stopPx")]
    pub stop_px: Option<Value>,
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "trailPct")]
    pub trail_pct: Option<Value>,
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "estimatedFees")]
    pub estimated_fees: Option<Vec<EstimatedFee>>,
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "exchangeLabel")]
    pub exchange_label: Option<String>,
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "feesExplanation")]
    pub fees_explanation: Option<FeesExplanation>,
    /// Received at the `/ordersimple/check` endpoint
    #[serde(rename = "estimatedBalance")]
    pub estimated_balance: Option<f64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderCheckResponse {
    pub acceptability_messages: Option<Vec<Message>>,
    pub check_order_data: OrderData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimatedFee {
    #[serde(rename = "type")]
    pub type_field: String,
    pub label: String,
    pub amount: f64,
    pub percentage: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeesExplanation {
    pub start_amount: String,
    pub product_fee: String,
    pub service_fee: String,
    pub scenarios: Vec<Message>,
    // pub translations: Translations,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub title: String,
    pub content: Vec<Vec<String>>,
}
