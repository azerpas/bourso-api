use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthParams {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountsParams {
    pub kind: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteParams {
    pub symbol: String,
    pub length: Option<i32>,
    pub interval: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderParams {
    pub account_id: String,
    pub symbol: String,
    pub quantity: usize,
    pub side: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionParams {
    pub account_id: String,
}
