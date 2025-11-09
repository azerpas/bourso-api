use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValueError {
    #[error("invalid client number: must be 8 digits (0-9)")]
    ClientNumber,
    #[error("invalid account id: must be 32 hexadecimal characters (0-9, a-f)")]
    AccountId,
    #[error("invalid symbol id: must be 6-12 alphanumeric characters (0-9, a-z, A-Z)")]
    SymbolId,
    #[error("invalid order quantity: must be a positive, non-zero integer")]
    OrderQuantity,
    #[error("invalid money amount: must be a positive, up to 2 decimal places float")]
    MoneyAmount,
    #[error("invalid transfer reason: must be 0-50 letters only (a-z, A-Z)")]
    TransferReason,
    #[error("invalid quote length: must be one of: 1, 5, 30, 90, 180, 365, 1825, 3650")]
    QuoteLength,
    #[error("invalid quote period: must be a positive integer")]
    QuotePeriod,
    #[error("invalid mfa code: must be 6-12 digits (0-9)")]
    MfaCode,
    #[error("invalid password: must be a non-empty string")]
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientNumber(String);
impl ClientNumber {
    pub fn new(s: &str) -> Result<Self, ValueError> {
        let t = s.trim();
        if t.len() == 8 && t.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(t.into()))
        } else {
            Err(ValueError::ClientNumber)
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for ClientNumber {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
impl AsRef<str> for ClientNumber {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(String);
impl AccountId {
    pub fn new(s: &str) -> Result<Self, ValueError> {
        let t = s.trim();
        if t.len() == 32 && t.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(Self(t.into()))
        } else {
            Err(ValueError::AccountId)
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for AccountId {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
impl AsRef<str> for AccountId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(String);
impl SymbolId {
    pub fn new(s: &str) -> Result<Self, ValueError> {
        let t = s.trim();
        if (6..=12).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric()) {
            Ok(Self(t.into()))
        } else {
            Err(ValueError::SymbolId)
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for SymbolId {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
impl AsRef<str> for SymbolId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderQuantity(u64);
impl OrderQuantity {
    pub fn new(v: u64) -> Result<Self, ValueError> {
        if v >= 1 {
            Ok(Self(v))
        } else {
            Err(ValueError::OrderQuantity)
        }
    }
    pub fn get(self) -> u64 {
        self.0
    }
}
impl FromStr for OrderQuantity {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u64 = s.parse().map_err(|_| ValueError::OrderQuantity)?;
        Self::new(v)
    }
}
