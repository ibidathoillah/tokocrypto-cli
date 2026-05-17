use clap::Subcommand;

use crate::errors::TokocryptoError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Get current account details (commissions, permissions)
    Info,

    /// Get non-zero asset balances
    Balance,

    /// Get details of a specific asset
    Assets {
        /// Asset name (e.g., ADA, BTC, USDT)
        asset: String,
    },

    /// Get your trade history for a symbol
    Trades {
        /// Trading pair symbol (e.g., BTC_USDT)
        symbol: String,

        /// Start from this trade ID
        #[arg(long)]
        from_id: Option<i64>,

        /// Limit number of trades (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
}

impl AccountCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, TokocryptoError> {
        let client = &ctx.client;

        let output = match self {
            Self::Info => {
                let result = client.get_signed("/open/v1/account/spot", &[]).await?;
                CommandOutput::new(result, "Account Info")
            }

            Self::Balance => {
                let result = client.get_signed("/open/v1/account/spot", &[]).await?;
                // Under data -> accountAssets
                if let Some(assets) = result.get("data").and_then(|d| d.get("accountAssets")) {
                    CommandOutput::new(serde_json::json!({ "accountAssets": assets }), "Balances")
                } else {
                    CommandOutput::new(result, "Balances")
                }
            }

            Self::Assets { asset } => {
                let asset_upper = asset.to_uppercase();
                let result = client
                    .get_signed("/open/v1/account/spot/asset", &[("asset", &asset_upper)])
                    .await?;
                CommandOutput::new(result, format!("Asset Info — {}", asset_upper))
            }

            Self::Trades {
                symbol,
                from_id,
                limit,
            } => {
                let sym = symbol.to_uppercase();
                let limit_str = limit.to_string();
                let from_id_str = from_id.map(|id| id.to_string());

                let mut params = vec![("symbol", sym.as_str()), ("limit", limit_str.as_str())];

                if let Some(ref fid) = from_id_str {
                    params.push(("fromId", fid.as_str()));
                    // Tokocrypto: "if field 'fromId' is defined, this field 'direct' becomes mandatory"
                    // direct: "prev" (ascending) or "next" (descending)
                    params.push(("direct", "next"));
                }

                let result = client.get_signed("/open/v1/orders/trades", &params).await?;
                CommandOutput::new(result, format!("Trade History — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}
