use crate::account::Transaction;
use crate::constants::BASE_URL;

use super::BoursoWebClient;

use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use tracing::debug;

lazy_static! {
    /// CSRF token embedded in the movement-export form.
    /// Matches: name="movementSearch[_token]" ... value="<token>"
    static ref EXPORT_TOKEN_REGEX: Regex =
        Regex::new(r#"movementSearch\[_token\]"[^>]*?value="(?P<token>[^"]+)""#)
            .expect("Failed to compile export token regex");
}

/// URL of the page hosting the movement-export form (source of the CSRF token).
const EXPORT_FORM_URL: &str = "/mon-budget/generate";
/// Endpoint the export form POSTs to.
const EXPORT_SUBMIT_PATH: &str = "/budget/exporter-mouvements";

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
        // Since 2026 the CSV export is a POST guarded by a per-session CSRF
        // token: GET the export form first, scrape `movementSearch[_token]`,
        // then POST the search. (The old GET-with-query flow now just 302s to
        // the HTML form page, which silently yielded zero transactions for
        // every account.)
        let token = self.get_export_token().await?;

        let form: Vec<(&str, &str)> = vec![
            ("movementSearch[label]", ""),
            ("movementSearch[selectedAccounts][]", account_id),
            ("movementSearch[fromDate]", from_date),
            ("movementSearch[toDate]", to_date),
            ("movementSearch[format]", "CSV"),
            ("movementSearch[filtredBy]", "filtredByCategory"),
            ("movementSearch[category]", ""),
            ("movementSearch[operationTypes]", ""),
            ("movementSearch[myBudgetPage]", "1"),
            ("movementSearch[operationType]", ""),
            ("movementSearch[_token]", token.as_str()),
            ("movementSearch[submit]", ""),
        ];

        let response = self
            .client
            .post(format!("{BASE_URL}{EXPORT_SUBMIT_PATH}"))
            .headers(self.get_headers())
            .form(&form)
            .send()
            .await?;

        // Follow redirects manually (the client uses Policy::none()); the CSV
        // download usually arrives after one 302.
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

        let status = response.status();
        debug!("Export response status: {}", status);

        let res = response.bytes().await?;
        let content = String::from_utf8_lossy(&res);
        // Strip BOM if present
        let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);

        // A non-empty export is CSV. A genuinely EMPTY period, though, bounces
        // (302) back to the export FORM page (/mon-budget/generate), which is
        // HTML and still carries the `movementSearch` form — that is the "no
        // operations" signal and is legitimate (e.g. a savings account with no
        // recent movement). Any other HTML (a login page, an error page: no
        // export form) means the session/flow broke — and we must NOT report
        // zero there, as silently returning an empty list has let callers
        // overwrite good data with nothing.
        if content.starts_with("<!DOCTYPE") || content.starts_with("<html") {
            let has_export_form = content.contains(r#"name="movementSearch""#)
                || content.contains("movementSearch[_token]");
            if has_export_form {
                debug!(
                    "No transactions for account {} from {} to {} \
                     (empty export bounced back to the form page)",
                    account_id, from_date, to_date
                );
                return Ok(Vec::new());
            }
            bail!(
                "Movement export for account {account_id} returned an HTML page \
                 (status {status}) with no export form — the BoursoBank session \
                 likely expired or the export flow changed. Refusing to report \
                 zero transactions."
            );
        }

        extract_transactions(content)
    }

    /// Fetch the export form page and scrape its `movementSearch[_token]` CSRF
    /// token, required to POST the movement export.
    #[cfg(not(tarpaulin_include))]
    async fn get_export_token(&self) -> Result<String> {
        let page = self
            .client
            .get(format!("{BASE_URL}{EXPORT_FORM_URL}"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;
        extract_export_token(&page)
    }
}

/// Extract the `movementSearch[_token]` CSRF token from the export form page.
fn extract_export_token(page: &str) -> Result<String> {
    EXPORT_TOKEN_REGEX
        .captures(page)
        .and_then(|c| c.name("token"))
        .map(|m| m.as_str().to_string())
        .context(
            "Could not find movementSearch[_token] on the export form page — \
             the export form layout may have changed or the session expired.",
        )
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
    fn test_extract_export_token() {
        let page = r#"<form name="movementSearch" method="post" action="/budget/exporter-mouvements">
            <input type="hidden" name="movementSearch[myBudgetPage]" value="1" >
            <input id="movementSearch__token" type="hidden" name="movementSearch[_token]" value="10987950e1775.DGNwm660QAUxguFToMcOnc6avhYKLj5.Xg4IzMPBODNA2rcS-It79ouv2FRZZXARF0Qb--Q" >
            </form>"#;
        let token = extract_export_token(page).unwrap();
        assert_eq!(
            token,
            "10987950e1775.DGNwm660QAUxguFToMcOnc6avhYKLj5.Xg4IzMPBODNA2rcS-It79ouv2FRZZXARF0Qb--Q"
        );
    }

    #[test]
    fn test_extract_export_token_missing() {
        assert!(extract_export_token("<html><body>no form here</body></html>").is_err());
    }

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
