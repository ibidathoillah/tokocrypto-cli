use clap::Subcommand;
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::errors::TokocryptoError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum WebSocketCommand {
    /// Stream order book depth updates
    Depth {
        /// Trading pair (e.g., BTC_USDT, TKO_BIDR)
        symbol: String,

        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,

        /// Manual symbol type override (1 - Main, 2 - Next, 3 - Nextme)
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Stream private order updates (requires API credentials)
    Orders {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,

        /// Symbol type for private listenKey stream (1 - Main, 3 - Nextme)
        #[arg(long, default_value = "1")]
        symbol_type: u32,
    },

    /// Stream private balance updates (requires API credentials)
    Balances {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,

        /// Symbol type for private listenKey stream (1 - Main, 3 - Nextme)
        #[arg(long, default_value = "1")]
        symbol_type: u32,
    },
}

impl WebSocketCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, TokocryptoError> {
        match self {
            Self::Depth {
                symbol,
                limit,
                seconds,
                symbol_type,
            } => {
                let bounds = StreamBounds::new(*limit, *seconds);
                stream_market_depth(ctx, symbol, *symbol_type, bounds).await?;
            }

            Self::Orders { limit, seconds, symbol_type } => {
                let bounds = StreamBounds::new(*limit, *seconds);
                stream_user(ctx, "orders", *symbol_type, bounds).await?;
            }

            Self::Balances { limit, seconds, symbol_type } => {
                let bounds = StreamBounds::new(*limit, *seconds);
                stream_user(ctx, "balances", *symbol_type, bounds).await?;
            }
        }

        Ok(CommandOutput::new(Value::Null, "").with_format(ctx.format))
    }
}

#[derive(Debug, Clone, Copy)]
struct StreamBounds {
    limit: Option<usize>,
    seconds: Option<u64>,
}

impl StreamBounds {
    fn new(limit: Option<usize>, seconds: Option<u64>) -> Self {
        Self { limit, seconds }
    }

    fn deadline(self) -> Option<Instant> {
        self.seconds
            .map(|seconds| Instant::now() + Duration::from_secs(seconds))
    }

    fn limit_reached(self, count: usize) -> bool {
        self.limit.is_some_and(|limit| count >= limit)
    }
}

async fn stream_market_depth(
    ctx: &AppContext,
    symbol: &str,
    symbol_type_opt: Option<u32>,
    bounds: StreamBounds,
) -> Result<(), TokocryptoError> {
    use colored::Colorize;

    let sym_type = match symbol_type_opt {
        Some(t) => t,
        None => crate::commands::market::detect_symbol_type(&ctx.client, symbol).await,
    };

    let ws_url = if sym_type == 1 {
        // Main symbols: lowercase, no underscores. Base: wss://stream-cloud.tokocrypto.site/ws
        let clean_symbol = symbol.replace("_", "").to_lowercase();
        format!("wss://stream-cloud.tokocrypto.site/ws/{}@depth", clean_symbol)
    } else if sym_type == 2 {
        // Next symbols: lowercase, kept underscores. Base: wss://www.tokocrypto.com/ws
        let clean_symbol = symbol.to_lowercase();
        format!("wss://www.tokocrypto.com/ws/{}@depth", clean_symbol)
    } else {
        // Nextme symbols: lowercase, no underscores. Base: wss://stream-toko.2meta.app/ws
        let clean_symbol = symbol.replace("_", "").to_lowercase();
        format!("wss://stream-toko.2meta.app/ws/{}@depth", clean_symbol)
    };

    eprintln!("{} Connecting to {} ...", "WS".cyan().bold(), ws_url);

    let (mut ws, _) = connect_async(&ws_url)
        .await
        .map_err(|e| TokocryptoError::WebSocket(e.to_string()))?;

    eprintln!("{} Subscribed to depth updates for {}", "WS".green().bold(), symbol.to_uppercase());

    let mut data_count = 0usize;
    let deadline = bounds.deadline();

    loop {
        let msg = match next_message(&mut ws, deadline).await? {
            Some(msg) => msg,
            None => break,
        };

        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    let output = CommandOutput::new(data, "Depth Update").with_format(ctx.format);
                    println!("{}", output.render());
                    data_count += 1;
                }
            }
            Ok(Message::Ping(payload)) => {
                let _ = ws.send(Message::Pong(payload)).await;
            }
            Ok(Message::Close(_)) => {
                eprintln!("{} Connection closed by remote server", "WS".yellow().bold());
                break;
            }
            Err(e) => {
                eprintln!("{} Error: {}", "WS".red().bold(), e);
                break;
            }
            _ => {}
        }

        if bounds.limit_reached(data_count) {
            break;
        }
    }

    Ok(())
}

async fn stream_user(
    ctx: &AppContext,
    channel_type: &str,
    symbol_type: u32,
    bounds: StreamBounds,
) -> Result<(), TokocryptoError> {
    use colored::Colorize;

    eprintln!("{} Requesting listenKey from Tokocrypto...", "WS".cyan().bold());

    // 1. Get listenKey (listenToken)
    let resp = ctx.client.post_signed("/open/v1/user-listen-token", &[]).await?;
    let listen_key = resp["data"]["token"]
        .as_str()
        .or_else(|| resp["data"]["listenKey"].as_str())
        .or_else(|| resp["data"]["listenToken"].as_str())
        .or_else(|| resp["listenKey"].as_str())
        .or_else(|| resp["listenToken"].as_str())
        .ok_or_else(|| TokocryptoError::Api {
            code: -1,
            message: format!("Failed to extract listenKey/listenToken from response: {}", resp),
        })?
        .to_string();

    // 2. Select User Stream WebSocket server
    let ws_url = if symbol_type == 3 {
        format!("wss://stream-toko.2meta.app/ws/{}", listen_key)
    } else {
        format!("wss://stream-cloud.tokocrypto.site/ws/{}", listen_key)
    };
    eprintln!("{} Connecting to user stream...", "WS".cyan().bold());

    let (mut ws, _) = connect_async(&ws_url)
        .await
        .map_err(|e| TokocryptoError::WebSocket(e.to_string()))?;

    eprintln!("{} Successfully connected to private stream", "WS".green().bold());

    // 3. Spawn a task to keep the listenKey alive (sent every 30 minutes)
    let client_clone = ctx.client.clone();
    let lk_clone = listen_key.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30 * 60)).await;
            // Ping / keepalive the listenKey
            // In Tokocrypto, we send a PUT to /open/v1/user-listen-token or similar
            let params = vec![
                ("listenKey", lk_clone.as_str()),
                ("listenToken", lk_clone.as_str()),
                ("token", lk_clone.as_str()),
            ];
            let _ = client_clone.put_public("/open/v1/user-listen-token", &params).await;
            eprintln!("{} Sent listenKey keep-alive", "WS".blue().bold());
        }
    });

    let mut data_count = 0usize;
    let deadline = bounds.deadline();

    loop {
        let msg = match next_message(&mut ws, deadline).await? {
            Some(msg) => msg,
            None => break,
        };

        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    let event = data["e"].as_str().unwrap_or("userDataUpdate").to_string();

                    // Filter based on requested channel
                    let matches_channel = match channel_type {
                        "orders" => event == "executionReport" || event.contains("Order"),
                        "balances" => event == "outboundAccountPosition" || event.contains("Balance"),
                        _ => true,
                    };

                    if matches_channel {
                        let output = CommandOutput::new(data, &event).with_format(ctx.format);
                        println!("{}", output.render());
                        data_count += 1;
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                let _ = ws.send(Message::Pong(payload)).await;
            }
            Ok(Message::Close(_)) => {
                eprintln!("{} Connection closed by server", "WS".yellow().bold());
                break;
            }
            Err(e) => {
                eprintln!("{} Error: {}", "WS".red().bold(), e);
                break;
            }
            _ => {}
        }

        if bounds.limit_reached(data_count) {
            break;
        }
    }

    Ok(())
}

async fn next_message<S>(
    ws: &mut S,
    deadline: Option<Instant>,
) -> Result<Option<S::Item>, TokocryptoError>
where
    S: Stream + Unpin,
{
    match deadline {
        Some(deadline) => {
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            timeout(deadline - now, ws.next())
                .await
                .map_or(Ok(None), Ok)
        }
        None => Ok(ws.next().await),
    }
}
