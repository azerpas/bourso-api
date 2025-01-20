use serde::{Deserialize, Serialize};

/// Type of account
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum AccountKind {
    Banking,
    Savings,
    Trading,
    Loans,
}

/// A bank account
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    /// Account id as an hexadecimal string (32 characters)
    pub id: String,
    /// Account name
    pub name: String,
    /// Balance in cents
    pub balance: isize,
    /// Account bank name as you can connect accounts from other banks
    pub bank_name: String,
    /// The type of account
    pub kind: AccountKind,
}
