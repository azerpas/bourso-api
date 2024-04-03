pub mod account;
pub mod client;
pub mod constants;

#[cfg(not(tarpaulin_include))]
pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}