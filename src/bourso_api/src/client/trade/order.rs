use anyhow::{Context, Result};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    account::{Account, AccountKind},
    client::config::Config,
};

use super::{get_trading_base_url, BoursoWebClient};

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
    ///
    /// # Returns
    /// Order ID and order price limit
    #[cfg(not(tarpaulin_include))]
    pub async fn order(
        &self,
        side: OrderSide,
        account: &Account,
        symbol: &str,
        quantity: usize,
        order_data: Option<OrderData>,
    ) -> Result<(String, Option<f64>)> {
        if account.kind != AccountKind::Trading {
            return Err(anyhow::anyhow!("Account is not a trading account"));
        }

        let response = self.prepare(account, symbol).await?;

        debug!("Prepare data {:#?}", response);

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
            order_data.order_expiration_date =
                Some(chrono::Utc::now().format("%Y-%m-%d").to_string());
        }

        order_data.resource_id = Some(response.resource_id);

        debug!("Order data: {:#?}", order_data);

        self.check(&order_data).await?;

        let response = self
            .confirm(&order_data.resource_id.as_ref().unwrap())
            .await?;

        info!(
            "Order for {} {} successfully passed with ID {} ✅",
            quantity, symbol, response.order_id
        );

        Ok((response.order_id, order_data.order_price_limit))
    }

    /// Prepare an order
    ///
    /// This will fetch trading data for the given symbol
    ///
    /// # Arguments
    ///
    /// * `account` - Account to use. Must be a trading account
    /// * `symbol` - Symbol to trade
    ///
    /// # Returns
    ///
    /// An order prepare response
    #[cfg(not(tarpaulin_include))]
    async fn prepare(&self, account: &Account, symbol: &str) -> Result<OrderPrepareResponse> {
        let url = get_order_prepare_url(&self.config, account, symbol)?;
        let response = self.client.get(url).send().await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!(
                "Failed to get order prepare response: {}",
                response
            ));
        }

        let response: OrderPrepareResponse = serde_json::from_str(&response).context(format!(
            "Failed to parse order prepare response. Response: {}",
            response
        ))?;

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
    #[cfg(not(tarpaulin_include))]
    async fn check(&self, data: &OrderData) -> Result<OrderCheckResponse> {
        let url = get_order_check_url(&self.config)?;
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(data)?)
            .send()
            .await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!(
                "Failed to get order check response: {}",
                response
            ));
        }

        let response: OrderCheckResponse = serde_json::from_str(&response).context(format!(
            "Failed to parse order check response. Response: {}",
            response
        ))?;

        Ok(response)
    }

    /// Confirm an order
    ///
    /// # Arguments
    ///
    /// * `resource_id` - Resource ID of the order to confirm
    ///
    /// # Returns
    ///
    /// An order confirm response
    #[cfg(not(tarpaulin_include))]
    async fn confirm(&self, resource_id: &str) -> Result<OrderConfirmResponse> {
        let url = get_order_confirm_url(&self.config)?;
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&serde_json::json!({
                "resourceId": resource_id
            }))?)
            .send()
            .await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 201 {
            return Err(anyhow::anyhow!(
                "Failed to get order confirm response: {}",
                response
            ));
        }

        let response: OrderConfirmResponse = serde_json::from_str(&response).context(format!(
            "Failed to parse order confirm response. Response: {}",
            response
        ))?;

        Ok(response)
    }

    /// Cancel an order that has not been executed yet
    ///
    /// # Arguments
    ///
    /// * `account` - Account to use. Must be a trading account
    /// * `order_id` - ID of the order to cancel
    #[cfg(not(tarpaulin_include))]
    pub async fn cancel_order(&self, account: &Account, order_id: &str) -> Result<()> {
        let url = get_cancel_order_url(&self.config)?;
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&serde_json::json!({
                "accountKey": &account.id,
                "reference": order_id
            }))?)
            .send()
            .await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!(
                "Failed to get order prepare response: {}",
                response
            ));
        }

        info!("Order {} successfully cancelled", order_id);

        Ok(())
    }
}

fn get_order_url(config: &Config) -> Result<String> {
    let trading_url = get_trading_base_url(config)?;

    Ok(format!("{}/order", trading_url))
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
    Ok(format!(
        "{}/ordersimple/check",
        get_trading_base_url(config)?
    ))
}

fn get_order_confirm_url(config: &Config) -> Result<String> {
    Ok(format!(
        "{}/ordersimple/confirm",
        get_trading_base_url(config)?
    ))
}

fn get_cancel_order_url(config: &Config) -> Result<String> {
    Ok(format!(
        "{}/orderdetail/cancel",
        get_trading_base_url(config)?
    ))
}

/// Data fetched from the `/order/prepare` endpoint
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPrepareResponse {
    /// ID of the order, will be used to confirm the order
    pub resource_id: String,
    pub is_pcc: bool,
    pub pcc_rights: PccRights,
    pub has_right_to_assign: bool,
    pub has_right_to_force: bool,
    /// Current position
    pub position: Position,
    /// Account used to place the order informations
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

/// Data fetched from the `/order/prepare` endpoint
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareOrderAccount {
    pub has_pfm: bool,
    /// The account RIB (Relevé d'Identité Bancaire)
    pub rib: String,
    /// The account IBAN (International Bank Account Number)
    pub iban: String,
    /// The account BIC (Bank Identifier Code)
    pub bic: String,
    /// The account number
    pub account_number: String,
    /// The account name
    pub name: String,
    /// The account balance in euros. More like the instant value of the account
    /// depending on the current market value of the assets and the cash balance
    pub balance: f64,
    pub internal: bool,
    /// The account currency
    pub currency: String,
    /// The account type (e.g PEA, PEA-PME, CTO, etc.)
    #[serde(rename = "type")]
    pub type_field: String,
    /// Is the account a professional account
    pub professional: bool,
    /// The account subtype (e.g ISA - Individual Savings Account, etc.)
    pub subtype: String,
    /// The account role (e.g titular)
    pub role: String,
    /// The account bank ID (e.g 1 for BoursoBank)
    pub bank_id: String,
    /// The account bank name (e.g BoursoBank)
    pub bank_name: String,
    pub cash_out: i64,
    pub cash_in: i64,
    pub account_key: String,
    pub pfm_account_key: Value,
    /// The account type category (e.g TRADING, SAVINGS, etc.)
    pub type_category: String,
    pub has_unregular_operations: bool,
    /// The account shortname
    pub short_name: String,
    /// Owned by a minor
    pub minor: bool,
    pub contact_id_owner: Value,
    /// KADOR is a special account for minors by BoursoBank
    #[serde(rename = "isKADOR")]
    pub is_kador: bool,
    pub profile_type: Value,
    pub details: Details,
    pub has_incident: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    /// The first time a cash transfer was made to the account
    pub first_cash_transfer_date: String,
    /// Gain/Losses in as a float value
    pub gain_losses_percent: f64,
    pub done_gain_losses_percent: f64,
    /// Current cash balance
    pub cash: f64,
    /// Current gain/losses in euros
    pub gain_losses: f64,
    pub done_gain_losses: f64,
    pub clearance_balance: f64,
    /// The account stocks value in euros
    pub stocks: f64,
    /// Today's date in format "2022-11-01"
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
    /// Exchange Label on which the symbol is traded (e.g Euronext Paris)
    pub exchange_label: String,
    /// Symbol ID (e.g 1rTPE500 for AMUNDI PEA S&P 500 ESG UCITS ETF)
    pub symbol: String,
    pub nb_decimals: i64,
    /// Symbol currency (e.g EUR)
    pub currency: String,
    pub label: String,
    /// ISIN (International Securities Identification Numbers) of the symbol (e.g FR0013412285)
    pub isin: String,
    /// Last price of the symbol
    pub last_price: f64,
    /// Morning Star key information document URL (e.g https://doc.morningstar.com/LatestDoc.aspx?clientid=boursorama&key=507703e53b7dec23&language=454&investmentid=F000013MGI&documenttype=299&market=1443&investmenttype=1&frame=0)
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

/// Data fetched from the `/order/prepare` endpoint and used to fill the default order data
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareOrderData {
    /// Minimum expiration date in format "2022-11-01"
    pub min_expire_tm: String,
    /// Maximum expiration date in format "2022-11-01"
    pub max_expire_tm: String,
    /// Invalid dates list in format "2022-11-01"
    pub invalid_dates_list: Vec<String>,
    /// List of order types per side (buy or sell)
    pub list_ord_type: ListOrdType,
    pub list_risk_md: Vec<String>,
    /// List of possible sides (buy or sell)
    pub side_list: Vec<OrderSide>,
    // pub config_ord_type: ConfigOrdType,
}

/// Possible order types per side (buy or sell)
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrdType {
    /// Buy order types
    pub b: Vec<OrderKind>,
    /// Sell order types
    pub s: Vec<OrderKind>,
}

/// Type of order
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum OrderKind {
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default, clap::ValueEnum)]
pub enum OrderSide {
    #[default]
    #[serde(rename = "B")]
    Buy,
    #[serde(rename = "S")]
    Sell,
}

/// Order data submitted to the `/ordersimple/check` endpoint
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
    pub acceptability_messages: Option<Vec<AcceptabilityMessage>>,
    pub check_order_data: OrderData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptabilityMessage {
    #[serde(rename = "type")]
    pub type_field: String,
    pub content: String,
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
    pub scenarios: Vec<ScenarioMessage>,
    // pub translations: Translations,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioMessage {
    pub title: String,
    pub content: Vec<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderConfirmResponse {
    pub order_id: String,
    pub order_state_label: String,
    pub ord_stat: String,
    pub action_message: ActionMessage,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub detail: Value,
    pub title: Value,
    pub body: Value,
    pub params: Value,
    pub category: Value,
    pub actions: Vec<Action>,
    pub flags: Vec<Value>,
    pub targets: Vec<Value>,
    pub visual_id: Value,
    pub visual_theme: Value,
    pub medias: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub label: Value,
    pub feature_id: Value,
    pub web: Value,
    pub api: ActionApi,
    pub disabled: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionApi {
    pub href: Value,
    pub method: Value,
    pub params: ActionApiParams,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionApiParams {
    pub account_type: String,
    pub account_key: String,
}
