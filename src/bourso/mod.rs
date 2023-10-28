pub mod account;
pub mod client;
pub mod constants;

pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}