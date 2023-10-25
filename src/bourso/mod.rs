pub mod constants;
pub mod client;
pub mod virtual_pad;
pub mod config;

/// Type of account
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccountKind {
    Banking,
    Savings,
    Trading,
    Loans,
}

/// A bank account
#[derive(Debug, Clone)]
pub struct Account {
    /// Account id as an hexadecimal string (32 characters)
    id: String,
    /// Account name
    name: String,
    /// Balance in cents
    balance: isize,
    /// Account bank name as you can connect accounts from other banks
    bank_name: String,
    /// The type of account
    kind: AccountKind,
}

pub fn get_client() -> client::BoursoWebClient {
    client::BoursoWebClient::new()
}