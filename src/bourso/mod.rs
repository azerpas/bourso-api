pub mod account;
pub mod client;
pub mod config;
pub mod constants;
pub mod virtual_pad;

pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}