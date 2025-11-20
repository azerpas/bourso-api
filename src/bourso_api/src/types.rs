use clap::ValueEnum;
use derive_more::{AsRef, From, Into};
use serde::{Deserialize, Serialize};
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
    #[error("invalid quote period: must be 0")]
    QuotePeriod,
    #[error("invalid mfa code: must be 6-12 digits (0-9)")]
    MfaCode,
    #[error("invalid password: must be a non-empty string")]
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef, From, Into)]
#[serde(try_from = "String", into = "String")]
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
}
impl FromStr for ClientNumber {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, From, Into)]
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
}
impl FromStr for AccountId {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, From, Into)]
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
}
impl FromStr for SymbolId {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRef, From, Into)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoneyAmount(f64);
impl MoneyAmount {
    pub fn new(v: f64) -> Result<Self, ValueError> {
        if v > 0.0 && v.fract().abs() <= 0.02 {
            Ok(Self(v))
        } else {
            Err(ValueError::MoneyAmount)
        }
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
impl FromStr for MoneyAmount {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: f64 = s.parse().map_err(|_| ValueError::MoneyAmount)?;
        Self::new(v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, From, Into)]
pub struct TransferReason(String);
impl TransferReason {
    pub fn new(s: &str) -> Result<Self, ValueError> {
        let t = s.trim();
        if t.len() > 50 {
            return Err(ValueError::TransferReason);
        }
        if !t.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(ValueError::TransferReason);
        }
        Ok(Self(t.into()))
    }
}
impl FromStr for TransferReason {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum QuoteLength {
    #[value(name = "1")]
    D1,
    #[value(name = "5")]
    D5,
    #[value(name = "30")]
    D30,
    #[value(name = "90")]
    D90,
    #[value(name = "180")]
    D180,
    #[value(name = "365")]
    D365,
    #[value(name = "1825")]
    D1825,
    #[value(name = "3650")]
    D3650,
}
impl QuoteLength {
    pub fn days(self) -> i64 {
        match self {
            QuoteLength::D1 => 1,
            QuoteLength::D5 => 5,
            QuoteLength::D30 => 30,
            QuoteLength::D90 => 90,
            QuoteLength::D180 => 180,
            QuoteLength::D365 => 365,
            QuoteLength::D1825 => 1825,
            QuoteLength::D3650 => 3650,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ValueEnum)]
pub enum OrderSide {
    #[serde(rename = "B")]
    Buy,
    #[serde(rename = "S")]
    Sell,
}

// TODO: support only 0 period for now, add support for other periods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuotePeriod(i64);
impl QuotePeriod {
    pub fn new(v: i64) -> Result<Self, ValueError> {
        if v == 0 {
            Ok(Self(v))
        } else {
            Err(ValueError::QuotePeriod)
        }
    }
    pub fn value(self) -> i64 {
        self.0
    }
}
impl FromStr for QuotePeriod {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: i64 = s.trim().parse().map_err(|_| ValueError::QuotePeriod)?;
        Self::new(v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, From, Into)]
pub struct MfaCode(String);
impl MfaCode {
    pub fn new(s: &str) -> Result<Self, ValueError> {
        let t = s.trim();
        if (6..=12).contains(&t.len()) && t.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(t.into()))
        } else {
            Err(ValueError::MfaCode)
        }
    }
}
impl FromStr for MfaCode {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef, From, Into)]
#[serde(try_from = "String", into = "String")]
pub struct Password(String);
impl Password {
    pub fn new(s: &str) -> Result<Self, ValueError> {
        let t = s.trim();
        if !t.is_empty() {
            Ok(Self(t.into()))
        } else {
            Err(ValueError::Password)
        }
    }
}
impl FromStr for Password {
    type Err = ValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
