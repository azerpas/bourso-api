use crate::api::{
    errors::ApiError,
    models::{AccountsParams, AuthParams, OrderParams, PositionParams, QuoteParams},
};
use actix_web::{web, HttpResponse};
use bourso_api::{
    account::{Account, AccountKind},
    client::{trade::order::OrderSide, BoursoWebClient},
    get_client,
};

pub async fn get_accounts(
    auth_params: web::Json<AuthParams>,
    accounts_params: web::Query<AccountsParams>,
) -> Result<HttpResponse, ApiError> {
    let mut web_client: BoursoWebClient = get_client();
    web_client.init_session().await?;
    web_client
        .login(&auth_params.username, &auth_params.password)
        .await?;

    let accounts: Vec<Account> = web_client
        .get_accounts(accounts_params.kind.as_ref().map(|k| match k.as_str() {
            "banking" => AccountKind::Banking,
            "saving" => AccountKind::Savings,
            "trading" => AccountKind::Trading,
            "loans" => AccountKind::Loans,
            _ => AccountKind::Banking, // FIXME
        }))
        .await?;
    Ok(HttpResponse::Ok().json(accounts))
}

pub async fn get_quote(
    quote_params: web::Query<QuoteParams>,
) -> Result<HttpResponse, ApiError> {
    let web_client: BoursoWebClient = get_client();
    let quotes = web_client
        .get_ticks(
            &quote_params.symbol,
            quote_params.length.unwrap_or(30) as i64,
            quote_params.interval.unwrap_or(0) as i64,
        )
        .await?;
    Ok(HttpResponse::Ok().json(quotes))
}

pub async fn new_order(
    auth_params: web::Json<AuthParams>,
    order_params: web::Json<OrderParams>,
) -> Result<HttpResponse, ApiError> {
    let mut web_client: BoursoWebClient = get_client();
    web_client.init_session().await?;
    web_client
        .login(&auth_params.username, &auth_params.password)
        .await?;

    let accounts = web_client.get_accounts(Some(AccountKind::Trading)).await?;
    let account = accounts
        .iter()
        .find(|a| a.id == order_params.account_id)
        .ok_or(ApiError::BadClientData)?;

    let side = match order_params.side.as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => return Err(ApiError::BadClientData),
    };

    let res = web_client
        .order(
            side,
            account,
            &order_params.symbol,
            order_params.quantity,
            None,
        )
        .await?;
    Ok(HttpResponse::Ok().json(res))
}

pub async fn get_positions(
    auth_params: web::Json<AuthParams>,
    position_params: web::Query<PositionParams>,
) -> Result<HttpResponse, ApiError> {
    let mut web_client: BoursoWebClient = get_client();
    web_client.init_session().await?;
    web_client
        .login(&auth_params.username, &auth_params.password)
        .await?;

    let accounts = web_client.get_accounts(Some(AccountKind::Trading)).await?;
    let account = accounts
        .iter()
        .find(|a| a.id == position_params.account_id)
        .ok_or(ApiError::BadClientData)?;

    let summary = web_client.get_trading_summary(account.clone()).await?;
    Ok(HttpResponse::Ok().json(summary))
}
