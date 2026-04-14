use crate::account::Transaction;
use crate::constants::BASE_URL;

use super::BoursoWebClient;

use anyhow::{Context, Result};
use tracing::debug;

impl BoursoWebClient {
    /// Get the transactions for an account over a date range.
    ///
    /// Uses the BoursoBank CSV export endpoint to retrieve transactions.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account ID (32-character hex string).
    /// * `from_date` - Start date in DD/MM/YYYY format.
    /// * `to_date` - End date in DD/MM/YYYY format.
    ///
    /// # Returns
    ///
    /// The transactions list as a vector of `Transaction`.
    #[cfg(not(tarpaulin_include))]
    pub async fn get_transactions(
        &self,
        account_id: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<Transaction>> {
        let response = self
            .client
            .get(format!("{BASE_URL}/budget/exporter-mouvements"))
            .query(&[
                ("movementSearch[selectedAccounts][]", account_id),
                ("movementSearch[fromDate]", from_date),
                ("movementSearch[toDate]", to_date),
                ("movementSearch[format]", "CSV"),
                ("movementSearch[filteredBy]", "filteredByCategory"),
                ("movementSearch[catergory]", ""),
                ("movementSearch[operationTypes]", ""),
                ("movementSearch[myBudgetPage]", "1"),
                ("movementSearch[submit]", ""),
            ])
            .headers(self.get_headers())
            .send()
            .await?;

        // Follow redirects manually (the client uses Policy::none())
        let response = if response.status() == 302 {
            let location = response
                .headers()
                .get("location")
                .context("Missing redirect location header")?
                .to_str()?;
            let redirect_url = if location.starts_with("http") {
                location.to_string()
            } else {
                format!("{BASE_URL}{location}")
            };
            debug!("Following redirect to {}", redirect_url);
            self.client
                .get(&redirect_url)
                .headers(self.get_headers())
                .send()
                .await?
        } else {
            response
        };

        debug!("Export response status: {}", response.status());

        let res = response.bytes().await?;
        let content = String::from_utf8_lossy(&res);
        // Strip BOM if present
        let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);

        // An HTML response means no transactions were found for the given period
        if content.starts_with("<!DOCTYPE") || content.starts_with("<html") {
            debug!(
                "No transactions found for account {} from {} to {}",
                account_id, from_date, to_date
            );
            return Ok(Vec::new());
        }

        extract_transactions(content)
    }
}

/// Parse a French-formatted amount string to f64.
///
/// Handles thousands separators (spaces and non-breaking spaces) and
/// comma decimal separators as used in BoursoBank CSV exports.
fn parse_amount(s: &str) -> f64 {
    let cleaned = s
        .trim()
        .replace('\u{a0}', "")
        .replace(' ', "")
        .replace(',', ".");
    if cleaned.is_empty() {
        return 0.0;
    }
    cleaned.parse::<f64>().unwrap_or(0.0)
}

/// Extract transactions from a BoursoBank CSV export string.
///
/// # Arguments
///
/// * `content` - The CSV content as a string (without BOM).
///
/// # Returns
///
/// The transactions list as a vector of `Transaction`.
fn extract_transactions(content: &str) -> Result<Vec<Transaction>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    reader
        .records()
        .map(|result| {
            let record = result.context("Failed to parse CSV record")?;
            Ok(Transaction {
                date_op: record.get(0).unwrap_or("").to_string(),
                date_val: record.get(1).unwrap_or("").to_string(),
                label: record.get(2).unwrap_or("").to_string(),
                category: record.get(3).unwrap_or("").to_string(),
                category_parent: record.get(4).unwrap_or("").to_string(),
                supplier_found: record.get(5).unwrap_or("").to_string(),
                amount: parse_amount(record.get(6).unwrap_or("")),
                comment: record.get(7).unwrap_or("").to_string(),
                account_num: record.get(8).unwrap_or("").to_string(),
                account_label: record.get(9).unwrap_or("").to_string(),
                account_balance: parse_amount(record.get(10).unwrap_or("")),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_amount() {
        assert_eq!(parse_amount("-568,13"), -568.13);
        assert_eq!(parse_amount("1 718,70"), 1718.70);
        assert_eq!(parse_amount("-8,99"), -8.99);
        assert_eq!(parse_amount("37.29"), 37.29);
        assert_eq!(parse_amount(""), 0.0);
        assert_eq!(parse_amount("  "), 0.0);
    }

    #[test]
    fn test_extract_transactions() {
        let transactions = extract_transactions(TRANSACTIONS_CSV).unwrap();
        assert_eq!(transactions.len(), 3);
        assert_eq!(transactions[0].date_op, "2026-02-09");
        assert_eq!(transactions[0].label, "VIR SEPA Loyer Villard");
        assert_eq!(transactions[0].amount, -568.13);
        assert_eq!(transactions[0].account_balance, 37.29);
        assert_eq!(transactions[0].category, "Virements émis");
        assert_eq!(transactions[1].date_op, "2026-02-06");
        assert_eq!(transactions[1].label, "CARTE 05/02/26 AMZN Mktp FR*308J CB*7686");
        assert_eq!(transactions[1].amount, -8.99);
        assert_eq!(transactions[2].label, "VIR SEPA FRANCE TRAVAIL");
        assert_eq!(transactions[2].amount, 1718.70);
        assert_eq!(transactions[2].account_balance, 629.41);
    }

    #[test]
    fn test_extract_transactions_empty_html() {
        let html = "<!DOCTYPE html><html><body>Error</body></html>";
        // HTML content should not be passed to extract_transactions
        // (handled by get_transactions), but let's verify it fails gracefully
        let result = extract_transactions(html);
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    pub const TRANSACTIONS_CSV: &str = r#"dateOp;dateVal;label;category;categoryParent;supplierFound;amount;comment;accountNum;accountLabel;accountbalance
2026-02-09;2026-02-09;"VIR SEPA Loyer Villard";"Virements émis";"Virements émis";"virement loyer villard";-568,13;;00040613484;BoursoBank;37.29
2026-02-06;2026-02-06;"CARTE 05/02/26 AMZN Mktp FR*308J CB*7686";"Livres, CD/DVD, bijoux, jouets…";"Vie quotidienne";amazon;-8,99;;00040613484;BoursoBank;605.42
2026-02-03;2026-02-03;"VIR SEPA FRANCE TRAVAIL";"Virements reçus";"Virements reçus";"virement france travail";1 718,70;;00040613484;BoursoBank;629.41
"#;
}
