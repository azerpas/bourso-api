pub mod constants;
pub mod client;
pub mod virtual_pad;

/// Type of account
#[derive(Debug, Clone)]
pub enum AccountKind {
    Banking,
    Savings,
    Trading,
}

/// A bank account
#[derive(Debug, Clone)]
pub struct Account {
    /// Account name
    name: String,
    /// Balance in cents
    balance: usize,
    /// Account bank name as you can connect accounts from other banks
    bank_name: String,
    /// The type of account
    kind: AccountKind,
}

pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}