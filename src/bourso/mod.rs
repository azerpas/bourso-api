pub const BASE_URL: &str = "https://clients.boursobank.com";

pub mod client;
pub mod virtual_pad;

pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}