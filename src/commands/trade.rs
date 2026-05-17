use clap::Subcommand;

use crate::errors::TokocryptoError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum OrderCommand {
    /// Place a buy order
    Buy {
        /// Trading pair symbol (e.g., TKO_BIDR, BTC_USDT)
        pair: String,

        /// Order type (LIMIT, MARKET, STOP_LOSS, STOP_LOSS_LIMIT, TAKE_PROFIT, TAKE_PROFIT_LIMIT, LIMIT_MAKER)
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity (base asset)
        #[arg(short = 'v', long)]
        volume: Option<String>,

        /// Spend amount in quote asset (for MARKET buy orders)
        #[arg(long)]
        quote_order_qty: Option<String>,

        /// Client order ID (optional)
        #[arg(long)]
        client_id: Option<String>,

        /// Time in force (GTC, IOC, FOK, GTX)
        #[arg(long)]
        time_in_force: Option<String>,

        /// Stop price (for stop loss / take profit orders)
        #[arg(long)]
        stop_price: Option<String>,

        /// Iceberg quantity (optional)
        #[arg(long)]
        iceberg_qty: Option<String>,
    },

    /// Place a sell order
    Sell {
        /// Trading pair symbol (e.g., TKO_BIDR, BTC_USDT)
        pair: String,

        /// Order type
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity (base asset)
        #[arg(short = 'v', long)]
        volume: Option<String>,

        /// Client order ID (optional)
        #[arg(long)]
        client_id: Option<String>,

        /// Time in force (GTC, IOC, FOK, GTX)
        #[arg(long)]
        time_in_force: Option<String>,

        /// Stop price (for stop loss / take profit orders)
        #[arg(long)]
        stop_price: Option<String>,

        /// Iceberg quantity (optional)
        #[arg(long)]
        iceberg_qty: Option<String>,
    },

    /// Cancel an active order
    Cancel {
        /// Order ID to cancel
        #[arg(long)]
        order_id: Option<i64>,

        /// Client order ID to cancel
        #[arg(long)]
        client_id: Option<String>,
    },

    /// Query a specific order's status
    Query {
        /// Order ID to query
        #[arg(long)]
        order_id: i64,

        /// Client order ID (optional)
        #[arg(long)]
        client_id: Option<String>,
    },

    /// List current open orders
    OpenOrders {
        /// Trading pair symbol (e.g., TKO_BIDR)
        pair: String,

        /// Limit number of open orders (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// List all orders (active, canceled, or filled)
    AllOrders {
        /// Trading pair symbol (e.g., TKO_BIDR)
        pair: String,

        /// Limit number of orders (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,

        /// Filter by order type (-1 = all, 1 = open, 2 = history)
        #[arg(short, long, default_value = "-1")]
        r#type: i32,

        /// Start from this order ID
        #[arg(long)]
        from_id: Option<String>,
    },

    /// Place a new OCO (One-Cancels-the-Other) order
    Oco {
        /// Trading pair symbol
        pair: String,

        /// Order side (BUY or SELL)
        #[arg(long)]
        side: String,

        /// Order quantity
        #[arg(short = 'v', long)]
        volume: String,

        /// Limit price
        #[arg(short, long)]
        price: String,

        /// Stop price
        #[arg(long)]
        stop_price: String,

        /// Stop limit price
        #[arg(long)]
        stop_limit_price: String,

        /// Client master order ID (optional)
        #[arg(long)]
        list_client_id: Option<String>,
    },
}

impl OrderCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, TokocryptoError> {
        let client = &ctx.client;

        let output = match self {
            Self::Buy {
                pair,
                r#type,
                price,
                volume,
                quote_order_qty,
                client_id,
                time_in_force,
                stop_price,
                iceberg_qty,
            } => {
                self.place_order(
                    ctx,
                    pair,
                    "BUY",
                    r#type,
                    price.as_deref(),
                    volume.as_deref(),
                    quote_order_qty.as_deref(),
                    client_id.as_deref(),
                    time_in_force.as_deref(),
                    stop_price.as_deref(),
                    iceberg_qty.as_deref(),
                )
                .await?
            }

            Self::Sell {
                pair,
                r#type,
                price,
                volume,
                client_id,
                time_in_force,
                stop_price,
                iceberg_qty,
            } => {
                self.place_order(
                    ctx,
                    pair,
                    "SELL",
                    r#type,
                    price.as_deref(),
                    volume.as_deref(),
                    None,
                    client_id.as_deref(),
                    time_in_force.as_deref(),
                    stop_price.as_deref(),
                    iceberg_qty.as_deref(),
                )
                .await?
            }

            Self::Cancel {
                order_id,
                client_id,
            } => {
                let mut params = Vec::new();
                let oid_str;
                if let Some(id) = order_id {
                    oid_str = id.to_string();
                    params.push(("orderId", oid_str.as_str()));
                }
                if let Some(ref cid) = client_id {
                    params.push(("clientId", cid.as_str()));
                }

                if params.is_empty() {
                    return Err(TokocryptoError::Validation(
                        "Either order-id or client-id must be provided".to_string(),
                    ));
                }

                let result = client
                    .post_signed("/open/v1/orders/cancel", &params)
                    .await?;
                CommandOutput::new(result, "Cancel Order Result")
            }

            Self::Query {
                order_id,
                client_id,
            } => {
                let oid_str = order_id.to_string();
                let mut params = vec![("orderId", oid_str.as_str())];
                if let Some(ref cid) = client_id {
                    params.push(("clientId", cid.as_str()));
                }

                let result = client.get_signed("/open/v1/orders/detail", &params).await?;
                CommandOutput::new(result, format!("Order Info — {}", order_id))
            }

            Self::OpenOrders { pair, count } => {
                let sym = crate::normalize_pair(pair);
                let limit_str = count.to_string();
                let params = vec![
                    ("symbol", sym.as_str()),
                    ("limit", limit_str.as_str()),
                    ("type", "1"), // 1 = Open
                ];

                let result = client.get_signed("/open/v1/orders", &params).await?;
                CommandOutput::new(result, format!("Open Orders — {}", sym))
            }

            Self::AllOrders {
                pair,
                count,
                r#type,
                from_id,
            } => {
                let sym = crate::normalize_pair(pair);
                let limit_str = count.to_string();
                let type_str = r#type.to_string();
                let mut params = vec![
                    ("symbol", sym.as_str()),
                    ("limit", limit_str.as_str()),
                    ("type", type_str.as_str()),
                ];

                if let Some(ref fid) = from_id {
                    params.push(("fromId", fid.as_str()));
                    params.push(("direct", "next"));
                }

                let result = client.get_signed("/open/v1/orders", &params).await?;
                CommandOutput::new(result, format!("All Orders — {}", sym))
            }

            Self::Oco {
                pair,
                side,
                volume,
                price,
                stop_price,
                stop_limit_price,
                list_client_id,
            } => {
                let sym = crate::normalize_pair(pair);
                let side_code = match side.to_uppercase().as_str() {
                    "BUY" => "0",
                    "SELL" => "1",
                    _ => side.as_str(),
                };

                let mut params = vec![
                    ("symbol", sym.as_str()),
                    ("side", side_code),
                    ("quantity", volume.as_str()),
                    ("price", price.as_str()),
                    ("stopPrice", stop_price.as_str()),
                    ("stopLimitPrice", stop_limit_price.as_str()),
                ];

                if let Some(ref lcid) = list_client_id {
                    params.push(("listClientId", lcid.as_str()));
                }

                let result = client.post_signed("/open/v1/orders/oco", &params).await?;
                CommandOutput::new(result, format!("OCO Order — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }

    #[allow(clippy::too_many_arguments)]
    async fn place_order(
        &self,
        ctx: &AppContext,
        symbol: &str,
        side: &str,
        order_type: &str,
        price: Option<&str>,
        quantity: Option<&str>,
        quote_order_qty: Option<&str>,
        client_id: Option<&str>,
        time_in_force: Option<&str>,
        stop_price: Option<&str>,
        iceberg_qty: Option<&str>,
    ) -> Result<CommandOutput, TokocryptoError> {
        let client = &ctx.client;
        let sym = crate::normalize_pair(symbol);

        let side_code = match side {
            "BUY" => "0",
            "SELL" => "1",
            _ => "0",
        };

        let type_upper = order_type.to_uppercase();
        let type_code = match type_upper.as_str() {
            "LIMIT" => "1",
            "MARKET" => "2",
            "STOP_LOSS" => "3",
            "STOP_LOSS_LIMIT" => "4",
            "TAKE_PROFIT" => "5",
            "TAKE_PROFIT_LIMIT" => "6",
            "LIMIT_MAKER" => "7",
            other => other,
        };

        let mut params = vec![
            ("symbol", sym.as_str()),
            ("side", side_code),
            ("type", type_code),
        ];

        if let Some(p) = price {
            params.push(("price", p));
        }
        if let Some(q) = quantity {
            params.push(("quantity", q));
        }
        if let Some(qoq) = quote_order_qty {
            params.push(("quoteOrderQty", qoq));
        }
        if let Some(cid) = client_id {
            params.push(("clientId", cid));
        }
        if let Some(sp) = stop_price {
            params.push(("stopPrice", sp));
        }
        if let Some(iq) = iceberg_qty {
            params.push(("icebergQty", iq));
        }

        let tif_upper;
        let tif_code;
        if let Some(tif) = time_in_force {
            tif_upper = tif.to_uppercase();
            tif_code = match tif_upper.as_str() {
                "GTC" => "1",
                "IOC" => "2",
                "FOK" => "3",
                "GTX" => "4",
                other => other,
            };
            params.push(("timeInForce", tif_code));
        }

        let result = client.post_signed("/open/v1/orders", &params).await?;

        let mut output = CommandOutput::new(result.clone(), "New Order Result");
        if let Some(data) = result.get("data") {
            if let Some(order_id) = data.get("orderId").and_then(|id| id.as_i64()) {
                output = output.with_addendum(format!(
                    "{} order placed successfully! Order ID: {}",
                    side, order_id
                ));
            }
        }

        Ok(output.with_format(ctx.format))
    }
}
