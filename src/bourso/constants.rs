pub const BASE_URL: &str = "https://clients.boursobank.com";
pub const SAVINGS_PATTERN: &str = r"(?ms)data-summary-savings>(.*?)</ul>";
pub const BANKING_PATTERN: &str = r"(?ms)data-summary-bank>(.*?)</div>";
pub const TRADING_PATTERN: &str = r"(?ms)data-summary-trading>(.*?)</div>";
pub const ACCOUNT_PATTERN: &str = r"(?ms)Solde\s:\s(?P<balance>[\d\s]+,?\d{0,2})\s€.+?c-info-box__account-label.+?>(?P<name>.+?)</span>.+?c-info-box__account-sub-label.+?>(?P<bank_name>.+?)</span>";