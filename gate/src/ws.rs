use crate::types::{PriceTick, GateError, PriceStream, OrderbookSnapshot, OrderbookLevel, OrderbookStream};
use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use std::collections::HashMap;

async fn get_symbol_types() -> Result<HashMap<String, u32>, GateError> {
    let url = "https://www.tokocrypto.com/open/v1/common/symbols";
    let resp = reqwest::Client::new().get(url).send().await
        .map_err(|e| GateError::Network(e.to_string()))?;
    let val: Value = resp.json().await
        .map_err(|e| GateError::Api(e.to_string()))?;
    let list = val.get("data").and_then(|d| d.get("list")).and_then(|v| v.as_array())
        .ok_or_else(|| GateError::Api("Missing data.list in common/symbols".to_string()))?;
    
    let mut map = HashMap::new();
    for item in list {
        if let Some(sym) = item["symbol"].as_str() {
            let t = item["type"].as_u64().unwrap_or(3) as u32;
            map.insert(sym.replace('_', "").to_uppercase(), t);
        }
    }
    Ok(map)
}

fn get_ws_host(symbol_type: u32) -> &'static str {
    match symbol_type {
        1 => "wss://stream-cloud.tokocrypto.site",
        2 => "wss://www.tokocrypto.com",
        _ => "wss://stream-toko.2meta.app",
    }
}

pub async fn connect_price_stream(pairs: &[String]) -> Result<PriceStream, GateError> {
    if pairs.is_empty() {
        return Err(GateError::InvalidSymbol("No pairs specified for price stream".to_string()));
    }

    let symbol_types = get_symbol_types().await.unwrap_or_default();
    
    // Group pairs by symbol type host
    let mut groups: HashMap<u32, Vec<String>> = HashMap::new();
    for pair in pairs {
        let clean = pair.replace(['_', '-', '/'], "").to_uppercase();
        let sym_type = symbol_types.get(&clean).copied().unwrap_or(3);
        groups.entry(sym_type).or_default().push(pair.clone());
    }

    let mut ws_streams = Vec::new();

    for (sym_type, group_pairs) in groups {
        let host = get_ws_host(sym_type);
        let mut streams = Vec::new();
        for pair in &group_pairs {
            // For type 2 (next), keep underscores in symbol name for WS
            let clean = if sym_type == 2 {
                pair.replace('/', "_").to_lowercase()
            } else {
                pair.replace(['_', '-', '/'], "").to_lowercase()
            };
            streams.push(format!("{}@trade", clean));
        }

        let url = format!("{}/stream?streams={}", host, streams.join("/"));
        let (ws_stream, _) = connect_async(&url).await
            .map_err(|e| GateError::WebSocket(format!("Failed to connect to {}: {}", url, e)))?;
        
        let (_, rx) = ws_stream.split();
        let pairs_clone = group_pairs.clone();
        
        let mapped = rx.filter_map(move |msg_res| {
            let pairs = pairs_clone.clone();
            async move {
                match msg_res {
                    Ok(Message::Text(text)) => {
                        let val: Value = serde_json::from_str(&text).ok()?;
                        let data = val.get("data")?;
                        let symbol_exchange = data.get("s").and_then(|v| v.as_str())?;
                        let price_str = data.get("p").and_then(|v| v.as_str())?;
                        let price: f64 = price_str.parse().ok()?;
                        let timestamp = data.get("T").and_then(|v| v.as_u64())?;
                        
                        let matched = pairs.iter().find(|p| {
                            let norm_p = p.replace(['_', '-', '/'], "").to_lowercase();
                            let norm_ex = symbol_exchange.replace('_', "").to_lowercase();
                            norm_p == norm_ex
                        })?;

                        Some(Ok(PriceTick {
                            symbol: matched.clone(),
                            price,
                            timestamp,
                        }))
                    }
                    Err(e) => Some(Err(GateError::WebSocket(e.to_string()))),
                    _ => None,
                }
            }
        });

        ws_streams.push(mapped.boxed());
    }

    if ws_streams.is_empty() {
        return Err(GateError::WebSocket("No WebSocket connections created".to_string()));
    }

    let merged = futures_util::stream::select_all(ws_streams);
    Ok(Box::pin(merged))
}

pub async fn connect_orderbook_stream(pair: &str, depth: usize) -> Result<OrderbookStream, GateError> {
    let symbol_types = get_symbol_types().await.unwrap_or_default();
    let clean_upper = pair.replace(['_', '-', '/'], "").to_uppercase();
    let sym_type = symbol_types.get(&clean_upper).copied().unwrap_or(3);
    
    let host = get_ws_host(sym_type);
    let clean = if sym_type == 2 {
        pair.replace('/', "_").to_lowercase()
    } else {
        pair.replace(['_', '-', '/'], "").to_lowercase()
    };

    let depth_level = match depth {
        d if d <= 5 => 5,
        d if d <= 10 => 10,
        _ => 20,
    };

    let url = format!("{}/ws/{}@depth{}", host, clean, depth_level);
    let (ws_stream, _) = connect_async(&url).await
        .map_err(|e| GateError::WebSocket(format!("Failed to connect to {}: {}", url, e)))?;
        
    let (_, rx) = ws_stream.split();
    let symbol = pair.to_string();

    let mapped = rx.filter_map(move |msg_res| {
        let symbol = symbol.clone();
        async move {
            match msg_res {
                Ok(Message::Text(text)) => {
                    let val: Value = serde_json::from_str(&text).ok()?;
                    let bids_val = val.get("bids").and_then(|v| v.as_array())?;
                    let asks_val = val.get("asks").and_then(|v| v.as_array())?;
                    
                    let parse_levels = |levels: &Vec<Value>| -> Vec<OrderbookLevel> {
                        levels.iter().filter_map(|level| {
                            let arr = level.as_array()?;
                            let price: f64 = arr.first()?.as_str()?.parse().ok()?;
                            let amount: f64 = arr.get(1)?.as_str()?.parse().ok()?;
                            Some(OrderbookLevel { price, amount })
                        }).collect()
                    };
                    
                    let bids = parse_levels(bids_val);
                    let asks = parse_levels(asks_val);
                    let timestamp = chrono::Utc::now().timestamp_millis() as u64;
                    
                    Some(Ok(OrderbookSnapshot {
                        symbol,
                        bids,
                        asks,
                        timestamp,
                    }))
                }
                Err(e) => Some(Err(GateError::WebSocket(e.to_string()))),
                _ => None,
            }
        }
    });

    Ok(Box::pin(mapped))
}
