pub mod account;
pub mod client;
pub mod constants;
pub mod types;

#[cfg(not(tarpaulin_include))]
pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}
