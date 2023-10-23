pub const BASE_URL: &str = "https://clients.boursobank.com";
pub const SAVINGS_PATTERN: &str = r"(?ms)data-summary-savings>(.*?)</ul>";
pub const BANKING_PATTERN: &str = r"(?ms)data-summary-bank>(.*?)</ul>";
pub const TRADING_PATTERN: &str = r"(?ms)data-summary-trading>(.*?)</ul>";
pub const BALANCE_PATTERN: &str = r"(?ms)Solde\s:\s(?P<balance>[\d\s]+,?\d{0,2})\s€";
pub const ACCOUNT_PATTERN: &str = r"(?ms)Solde\s:\s(?P<balance>[\d\s]+,?\d{0,2})\s€.+?c-info-box__account-label.+?>(?P<name>.+?)</span>.+?c-info-box__account-sub-label.+?>(?P<bank_name>.+?)</span>";