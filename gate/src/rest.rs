use crate::types::{MarketPair, PriceTick, GateError};
use reqwest::Client;
use serde_json::Value;

pub struct RestClient {
    client: Client,
}

impl RestClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn list_pairs(&self) -> Result<Vec<MarketPair>, GateError> {
        let url = "https://www.tokocrypto.com/open/v1/common/symbols";
        let resp = self.client.get(url).send().await
            .map_err(|e| GateError::Network(e.to_string()))?;
        
        let val: Value = resp.json().await
            .map_err(|e| GateError::Api(e.to_string()))?;
            
        let list = val.get("data").and_then(|d| d.get("list")).and_then(|v| v.as_array())
            .ok_or_else(|| GateError::Api("Missing data.list in common/symbols".to_string()))?;
            
        let mut pairs = Vec::new();
        for sym_val in list {
            let symbol_raw = sym_val.get("symbol").and_then(|v| v.as_str()).unwrap_or_default();
            let base = sym_val.get("baseAsset").and_then(|v| v.as_str()).unwrap_or_default();
            let quote = sym_val.get("quoteAsset").and_then(|v| v.as_str()).unwrap_or_default();
            
            if !symbol_raw.is_empty() && !base.is_empty() && !quote.is_empty() {
                pairs.push(MarketPair {
                    symbol: format!("{}/{}", base.to_uppercase(), quote.to_uppercase()),
                    base: base.to_uppercase(),
                    quote: quote.to_uppercase(),
                    active: true, // Tokocrypto lists active symbols in this endpoint
                });
            }
        }
        Ok(pairs)
    }

    pub async fn last_price(&self, pair: &str) -> Result<PriceTick, GateError> {
        let clean = pair.replace(['_', '-', '/'], "").to_uppercase();
        let site_url = format!("https://www.tokocrypto.site/api/v3/ticker/price?symbol={}", clean);
        
        if let Ok(resp) = self.client.get(&site_url).send().await {
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(price_str) = val.get("price").and_then(|v| v.as_str()) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        return Ok(PriceTick {
                            symbol: pair.to_string(),
                            price,
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                        });
                    }
                }
            }
        }

        // Fallback: Query /open/v1/market/trades from www.tokocrypto.com
        let raw_symbol = pair.replace('/', "_").to_uppercase();
        let fallback_url = format!("https://www.tokocrypto.com/open/v1/market/trades?symbol={}&limit=1", raw_symbol);
        
        let resp = self.client.get(&fallback_url).send().await
            .map_err(|e| GateError::Network(e.to_string()))?;
            
        let val: Value = resp.json().await
            .map_err(|e| GateError::Api(e.to_string()))?;
            
        let list = val.get("data").and_then(|d| d.get("list")).and_then(|v| v.as_array())
            .ok_or_else(|| GateError::Api("Missing data.list in market/trades response".to_string()))?;
            
        let first = list.first()
            .ok_or_else(|| GateError::Api("No trades found to retrieve price".to_string()))?;
            
        let price_str = first.get("price").or_else(|| first.get("p")).and_then(|v| v.as_str())
            .ok_or_else(|| GateError::Api("Missing price field in trade data".to_string()))?;
            
        let price: f64 = price_str.parse()
            .map_err(|e| GateError::Api(format!("Invalid trade price ({}): {}", price_str, e)))?;
            
        let ts = first.get("time").or_else(|| first.get("t")).and_then(|v| v.as_u64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() as u64);
            
        Ok(PriceTick {
            symbol: pair.to_string(),
            price,
            timestamp: ts,
        })
    }
}
