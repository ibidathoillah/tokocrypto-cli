use clap::Subcommand;

use crate::errors::TokocryptoError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    /// Test connectivity to the REST API
    Ping,

    /// Get the current server time
    ServerTime,

    /// Get supported trading symbols and rules
    Symbols,

    /// Query symbol execution rules (Price Range)
    ExecutionRules {
        /// Query for a single symbol
        #[arg(long)]
        symbol: Option<String>,

        /// Query for multiple symbols (comma separated)
        #[arg(long)]
        symbols: Option<String>,

        /// Filter by symbol status (TRADING, HALT, BREAK)
        #[arg(long)]
        status: Option<String>,
    },

    /// Get order book depth
    Depth {
        /// Trading pair symbol (e.g., BTC_USDT or TKO_BIDR)
        symbol: String,

        /// Limit number of price levels (default: 100, max: 5000, valid: 5, 10, 20, 50, 100, 500)
        #[arg(short, long, default_value = "100")]
        limit: u32,

        /// Manual symbol type override (1 - Main, 2 - Next, 3 - Nextme)
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Get recent trades
    Trades {
        /// Trading pair symbol
        symbol: String,

        /// Start from this trade ID
        #[arg(long)]
        from_id: Option<i64>,

        /// Limit number of trades (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,

        /// Manual symbol type override
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Get compressed/aggregate trades list
    AggTrades {
        /// Trading pair symbol
        symbol: String,

        /// Trade ID to fetch from
        #[arg(long)]
        from_id: Option<i64>,

        /// Start time in milliseconds
        #[arg(long)]
        start_time: Option<i64>,

        /// End time in milliseconds
        #[arg(long)]
        end_time: Option<i64>,

        /// Limit number of trades (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,

        /// Manual symbol type override
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Get kline/candlestick data bars
    Klines {
        /// Trading pair symbol
        symbol: String,

        /// Chart interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1h")]
        interval: String,

        /// Start time in milliseconds
        #[arg(long)]
        start_time: Option<i64>,

        /// End time in milliseconds
        #[arg(long)]
        end_time: Option<i64>,

        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,

        /// Manual symbol type override
        #[arg(long)]
        symbol_type: Option<u32>,
    },
}

impl MarketCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, TokocryptoError> {
        let client = &ctx.client;

        let output = match self {
            Self::Ping => {
                let result = client.get_public("/open/v1/common/time", &[]).await?;
                CommandOutput::new(result, "Ping").with_addendum("Tokocrypto API is reachable")
            }

            Self::ServerTime => {
                let result = client.get_public("/open/v1/common/time", &[]).await?;
                let ts = result["timestamp"]
                    .as_u64()
                    .or_else(|| result["data"].as_u64())
                    .unwrap_or(0);
                let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| ts.to_string());

                CommandOutput::new(result, "Server Time").with_addendum(format!("{} ({})", dt, ts))
            }

            Self::Symbols => {
                let result = client.get_public("/open/v1/common/symbols", &[]).await?;
                CommandOutput::new(result, "Supported Trading Symbols")
            }

            Self::ExecutionRules {
                symbol,
                symbols,
                status,
            } => {
                let mut params = Vec::new();
                let s_str;
                let ss_str;
                let stat_str;
                if let Some(s) = symbol {
                    s_str = crate::normalize_pair(s);
                    params.push(("symbol", s_str.as_str()));
                } else if let Some(ss) = symbols {
                    ss_str = crate::normalize_pair(ss);
                    params.push(("symbols", ss_str.as_str()));
                } else if let Some(st) = status {
                    stat_str = st.to_uppercase();
                    params.push(("symbolStatus", stat_str.as_str()));
                }

                // Base endpoint override is https://www.tokocrypto.site
                let endpoint =
                    format!("{}/api/v3/executionRules", crate::config::DEFAULT_SITE_HOST);
                let result = client.get_public(&endpoint, &params).await?;
                CommandOutput::new(result, "Execution Rules (Price Range)")
            }

            Self::Depth {
                symbol,
                limit,
                symbol_type,
            } => {
                let sym_type = match symbol_type {
                    Some(t) => *t,
                    None => detect_symbol_type(client, symbol).await,
                };

                let limit_str = limit.to_string();
                let result = if sym_type == 1 {
                    // Symbol type 1 uses https://www.tokocrypto.site/api/v3/depth
                    // Replace _ of symbol with empty string
                    let clean_symbol = crate::normalize_pair(symbol);
                    let endpoint = format!("{}/api/v3/depth", crate::config::DEFAULT_SITE_HOST);
                    client
                        .get_public(
                            &endpoint,
                            &[("symbol", &clean_symbol), ("limit", &limit_str)],
                        )
                        .await?
                } else {
                    // Symbol type 3 uses https://cloudme-toko.2meta.app/api/v1/depth
                    let clean_symbol = crate::normalize_pair(symbol);
                    let endpoint = format!("{}/api/v1/depth", crate::config::CLOUDME_HOST);
                    client
                        .get_public(
                            &endpoint,
                            &[("symbol", &clean_symbol), ("limit", &limit_str)],
                        )
                        .await?
                };

                CommandOutput::new(
                    result,
                    format!("Order Book Depth — {}", symbol.to_uppercase()),
                )
            }

            Self::Trades {
                symbol,
                from_id,
                limit,
                symbol_type,
            } => {
                let sym_type = match symbol_type {
                    Some(t) => *t,
                    None => detect_symbol_type(client, symbol).await,
                };

                let limit_str = limit.to_string();
                let from_id_str = from_id.map(|id| id.to_string());
                let mut params = vec![("limit", limit_str.as_str())];

                let result = if sym_type == 1 {
                    let clean_symbol = crate::normalize_pair(symbol);
                    params.push(("symbol", clean_symbol.as_str()));
                    if let Some(ref fid) = from_id_str {
                        params.push(("fromId", fid.as_str()));
                    }
                    let endpoint = format!("{}/api/v3/trades", crate::config::DEFAULT_SITE_HOST);
                    client.get_public(&endpoint, &params).await?
                } else {
                    let clean_symbol = crate::normalize_pair(symbol);
                    params.push(("symbol", clean_symbol.as_str()));
                    if let Some(ref fid) = from_id_str {
                        params.push(("fromId", fid.as_str()));
                    }
                    client.get_public("/open/v1/market/trades", &params).await?
                };

                CommandOutput::new(result, format!("Recent Trades — {}", symbol.to_uppercase()))
            }

            Self::AggTrades {
                symbol,
                from_id,
                start_time,
                end_time,
                limit,
                symbol_type,
            } => {
                let sym_type = match symbol_type {
                    Some(t) => *t,
                    None => detect_symbol_type(client, symbol).await,
                };

                let limit_str = limit.to_string();
                let from_id_str = from_id.map(|id| id.to_string());
                let start_time_str = start_time.map(|t| t.to_string());
                let end_time_str = end_time.map(|t| t.to_string());

                let mut params = vec![("limit", limit_str.as_str())];
                if let Some(ref fid) = from_id_str {
                    params.push(("fromId", fid.as_str()));
                }
                if let Some(ref st) = start_time_str {
                    params.push(("startTime", st.as_str()));
                }
                if let Some(ref et) = end_time_str {
                    params.push(("endTime", et.as_str()));
                }

                let result = if sym_type == 1 {
                    let clean_symbol = crate::normalize_pair(symbol);
                    params.push(("symbol", clean_symbol.as_str()));
                    let endpoint = format!("{}/api/v3/aggTrades", crate::config::DEFAULT_SITE_HOST);
                    client.get_public(&endpoint, &params).await?
                } else {
                    let clean_symbol = crate::normalize_pair(symbol);
                    params.push(("symbol", clean_symbol.as_str()));
                    let endpoint = format!("{}/api/v1/aggTrades", crate::config::CLOUDME_HOST);
                    client.get_public(&endpoint, &params).await?
                };

                CommandOutput::new(
                    result,
                    format!("Aggregate Trades — {}", symbol.to_uppercase()),
                )
            }

            Self::Klines {
                symbol,
                interval,
                start_time,
                end_time,
                limit,
                symbol_type,
            } => {
                let sym_type = match symbol_type {
                    Some(t) => *t,
                    None => detect_symbol_type(client, symbol).await,
                };

                let limit_str = limit.to_string();
                let start_time_str = start_time.map(|t| t.to_string());
                let end_time_str = end_time.map(|t| t.to_string());

                let mut params = vec![
                    ("interval", interval.as_str()),
                    ("limit", limit_str.as_str()),
                ];
                if let Some(ref st) = start_time_str {
                    params.push(("startTime", st.as_str()));
                }
                if let Some(ref et) = end_time_str {
                    params.push(("endTime", et.as_str()));
                }

                let result = if sym_type == 1 {
                    let clean_symbol = crate::normalize_pair(symbol);
                    params.push(("symbol", clean_symbol.as_str()));
                    let endpoint = format!("{}/api/v3/klines", crate::config::DEFAULT_SITE_HOST);
                    client.get_public(&endpoint, &params).await?
                } else {
                    let clean_symbol = crate::normalize_pair(symbol);
                    params.push(("symbol", clean_symbol.as_str()));
                    let endpoint = format!("{}/api/v1/klines", crate::config::CLOUDME_HOST);
                    client.get_public(&endpoint, &params).await?
                };

                CommandOutput::new(result, format!("Klines — {}", symbol.to_uppercase()))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}

/// Helper function to detect symbol type via GET /open/v1/common/symbols
pub async fn detect_symbol_type(client: &crate::client::TokocryptoClient, symbol: &str) -> u32 {
    let sym_upper = symbol.to_uppercase();
    let clean_upper = crate::normalize_pair(symbol);
    if let Ok(res) = client.get_public("/open/v1/common/symbols", &[]).await {
        if let Some(list) = res["data"]["list"].as_array() {
            for item in list {
                if let Some(s) = item["symbol"].as_str() {
                    let s_upper = s.to_uppercase();
                    if s_upper == sym_upper || crate::normalize_pair(s) == clean_upper {
                        return item["type"].as_u64().unwrap_or(3) as u32;
                    }
                }
            }
        }
    }
    // Default fallback
    3
}
