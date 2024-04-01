use serde::{Serialize, Deserialize};
use anyhow::{Context, Result};

use crate::client::BoursoWebClient;

impl BoursoWebClient {

    /// Get the ticks for a given symbol, length and period
    /// 
    /// Ticks are quotes for a given symbol, period (time period interval) and length (the time frame)
    /// 
    /// # Arguments
    /// 
    /// * `symbol` - The symbol id of the stock (e.g: '1rTCW8')
    /// * `length` - The length period of the stock (e.g: '30' for 30 days)
    /// * `period` - The interval of the stock (e.g: '0' for default interval)
    /// 
    /// # Returns
    /// 
    /// A struct containing the quotes for the given symbol, period and length
    #[cfg(not(tarpaulin_include))]
    pub async fn get_ticks(&self, symbol: &str, length: i64, period: i64) -> Result<GetTicksEOD> {
        let url = format!(
            "https://www.boursorama.com/bourse/action/graph/ws/GetTicksEOD?symbol={}&length={}&period={}&guid=",
            symbol,
            length,
            period
        );

        let response = self.client.get(&url)
            .header("Content-Type", "application/json;charset=UTF-8")
            .send()
            .await?;

        let status_code = response.status();

        let response = response.text().await?;

        if status_code != 200 {
            return Err(anyhow::anyhow!("Failed to get ticks response ({}): {}", status_code, response));
        }
        
        let response: GetTicksEOD = serde_json::from_str(&response)
            .context("Failed to parse get ticks response")?;

        Ok(response)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTicksEOD {
    pub d: D,
}

/// Quotes for a given symbol, period (time period interval) and length (the time frame)
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct D {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "SymbolId")]
    pub symbol_id: String,
    #[serde(rename = "Xperiod")]
    pub xperiod: i64,
    #[serde(rename = "QuoteTab")]
    pub quote_tab: Vec<QuoteTab>,
    #[serde(rename = "qv")]
    pub second_to_last_quote: Option<QuoteTab>,
    #[serde(rename = "qd")]
    pub last_quote: Option<QuoteTab>,
}

impl D {
    /// Get previous day's quote
    pub fn get_last_quote(&self) -> Option<QuoteTab> {
        self.last_quote.clone()
    }

    /// Get the second to last day's quote
    pub fn get_second_to_last_quote(&self) -> Option<QuoteTab> {
        self.second_to_last_quote.clone()
    }

    /// Get all the quotes for the given period and length
    pub fn get_quotes(&self) -> Vec<QuoteTab> {
        self.quote_tab.clone()
    }

    /// Get the highest value of the quotes for the given period and length
    pub fn get_highest_value(&self) -> f64 {
        self.quote_tab.iter().map(|quote| quote.high).fold(0.0, f64::max)
    }

    /// Get the lowest value of the quotes for the given period and length
    pub fn get_lowest_value(&self) -> f64 {
        self.quote_tab.iter().map(|quote| quote.low).fold(f64::MAX, f64::min)
    }

    /// Get the average value of the quotes on closing for the given period and length
    pub fn get_average_value(&self) -> f64 {
        let sum: f64 = self.quote_tab.iter().map(|quote| quote.close).sum();
        sum / self.quote_tab.len() as f64
    }

    /// Get the volume of the quotes for the given period and length
    pub fn get_volume(&self) -> i64 {
        self.quote_tab.iter().map(|quote| quote.volume).sum()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteTab {
    #[serde(rename = "d")]
    pub date: i64,
    #[serde(rename = "o")]
    pub open: f64,
    #[serde(rename = "h")]
    pub high: f64,
    #[serde(rename = "l")]
    pub low: f64,
    #[serde(rename = "c")]
    pub close: f64,
    #[serde(rename = "v")]
    pub volume: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ticks() {
        let response = GetTicksEOD {
            d: D {
                name: "Amundi MSCI World Dist".to_string(),
                symbol_id: "1rTEWLD".to_string(),
                xperiod: 0,
                quote_tab: vec![
                    QuoteTab {
                        date: 19786,
                        open: 29.39,
                        high: 29.448,
                        low: 29.31,
                        close: 29.363,
                        volume: 55638,
                    },
                    QuoteTab {
                        date: 19787,
                        open: 29.342,
                        high: 29.349,
                        low: 29.111,
                        close: 29.17,
                        volume: 35539,
                    },
                    QuoteTab {
                        date: 19788,
                        open: 29.179,
                        high: 29.318,
                        low: 29.175,
                        close: 29.265,
                        volume: 27496,
                    },
                    QuoteTab {
                        date: 19789,
                        open: 29.176,
                        high: 29.492,
                        low: 29.125,
                        close: 29.397,
                        volume: 62962,
                    },
                    QuoteTab {
                        date: 19790,
                        open: 29.457,
                        high: 29.569,
                        low: 29.4,
                        close: 29.408,
                        volume: 43272,
                    },
                    QuoteTab {
                        date: 19793,
                        open: 29.183,
                        high: 29.239,
                        low: 29.1,
                        close: 29.2,
                        volume: 72401,
                    },
                    QuoteTab {
                        date: 19794,
                        open: 29.328,
                        high: 29.547,
                        low: 29.25,
                        close: 29.441,
                        volume: 21226,
                    },
                    QuoteTab {
                        date: 19795,
                        open: 29.503,
                        high: 29.539,
                        low: 29.441,
                        close: 29.461,
                        volume: 15609,
                    },
                    QuoteTab {
                        date: 19796,
                        open: 29.507,
                        high: 29.6,
                        low: 29.407,
                        close: 29.431,
                        volume: 23060,
                    },
                    QuoteTab {
                        date: 19797,
                        open: 29.36,
                        high: 29.36,
                        low: 29.36,
                        close: 29.36,
                        volume: 500,
                    },
                    QuoteTab {
                        date: 19800,
                        open: 30.0,
                        high: 30.0,
                        low: 29.5,
                        close: 29.63,
                        volume: 880,
                    },
                    QuoteTab {
                        date: 19801,
                        open: 29.52,
                        high: 29.895,
                        low: 29.215,
                        close: 29.535,
                        volume: 2723,
                    },
                    QuoteTab {
                        date: 19802,
                        open: 29.63,
                        high: 29.86,
                        low: 29.54,
                        close: 29.665,
                        volume: 8295,
                    },
                    QuoteTab {
                        date: 19803,
                        open: 29.91,
                        high: 30.375,
                        low: 29.56,
                        close: 30.07,
                        volume: 24456,
                    },
                    QuoteTab {
                        date: 19804,
                        open: 30.08,
                        high: 30.2,
                        low: 30.0,
                        close: 30.075,
                        volume: 14441,
                    },
                    QuoteTab {
                        date: 19807,
                        open: 30.025,
                        high: 30.08,
                        low: 29.9,
                        close: 29.97,
                        volume: 20730,
                    },
                    QuoteTab {
                        date: 19808,
                        open: 29.995,
                        high: 30.09,
                        low: 29.95,
                        close: 30.045,
                        volume: 13360,
                    },
                    QuoteTab {
                        date: 19809,
                        open: 30.025,
                        high: 30.134,
                        low: 30.0,
                        close: 30.035,
                        volume: 25590,
                    },
                    QuoteTab {
                        date: 19810,
                        open: 30.204,
                        high: 30.279,
                        low: 30.176,
                        close: 30.225,
                        volume: 16939,
                    },
                ],
                second_to_last_quote: None,
                last_quote: None,
            },
        };

        assert_eq!(response.d.name, "Amundi MSCI World Dist");
        assert_eq!(response.d.symbol_id, "1rTEWLD");
        assert_eq!(response.d.xperiod, 0);
        assert_eq!(response.d.quote_tab.len(), 19);
        assert_eq!(response.d.get_highest_value(), 30.375);
        assert_eq!(response.d.get_lowest_value(), 29.1);
        assert_eq!(response.d.get_average_value(), 29.618210526315796);
        assert_eq!(response.d.get_volume(), 485117);
    }
}
