use serde::{Deserialize, Serialize};

/// Type of account
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum AccountKind {
    Banking,
    Savings,
    #[default]
    Trading,
    Loans,
}

/// A bank account
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

/// A bank transaction
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Transaction {
    /// Operation date (YYYY-MM-DD)
    pub date_op: String,
    /// Value date (YYYY-MM-DD)
    pub date_val: String,
    /// Transaction label/description
    pub label: String,
    /// Transaction category
    pub category: String,
    /// Parent category
    pub category_parent: String,
    /// Supplier found by BoursoBank
    pub supplier_found: String,
    /// Transaction amount in EUR
    pub amount: f64,
    /// User comment
    pub comment: String,
    /// Account number
    pub account_num: String,
    /// Account label
    pub account_label: String,
    /// Account balance after transaction in EUR
    pub account_balance: f64,
}
