pub mod auth;
pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod mcp;
pub mod output;

use clap::{Parser, Subcommand};

use crate::client::TokocryptoClient;
use crate::commands::{
    account, auth as auth_cmds, funding, market, paper, trade, utility, websocket,
};
use crate::errors::TokocryptoError;
use crate::output::{CommandOutput, OutputFormat};

pub(crate) fn normalize_pair(pair: &str) -> String {
    pair.replace(['_', '-', '/'], "").to_uppercase()
}

pub(crate) fn normalize_pair_list(pairs: &str) -> String {
    pairs
        .split(',')
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .map(normalize_pair)
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn normalize_pair_ws(pair: &str, symbol_type: u32) -> String {
    let compact = normalize_pair(pair).to_lowercase();
    if symbol_type == 2 {
        for quote in ["bidr", "usdt", "idr", "btc", "eth", "bnb"] {
            if let Some(base) = compact.strip_suffix(quote) {
                if !base.is_empty() {
                    return format!("{}_{}", base, quote);
                }
            }
        }
    }
    compact
}

#[cfg(test)]
mod pair_tests {
    use super::*;

    #[test]
    fn normalizes_pair_for_api() {
        assert_eq!(normalize_pair("TKOIDR"), "TKOIDR");
        assert_eq!(normalize_pair("tko_idr"), "TKOIDR");
        assert_eq!(normalize_pair("tko-idr"), "TKOIDR");
        assert_eq!(normalize_pair("tko/idr"), "TKOIDR");
    }

    #[test]
    fn normalizes_pair_list_for_api() {
        assert_eq!(normalize_pair_list("tko_idr, btc-usdt"), "TKOIDR,BTCUSDT");
    }

    #[test]
    fn normalizes_pair_for_websocket() {
        assert_eq!(normalize_pair_ws("TKO_IDR", 1), "tkoidr");
        assert_eq!(normalize_pair_ws("TKO-IDR", 2), "tko_idr");
        assert_eq!(normalize_pair_ws("TKOIDR", 2), "tko_idr");
    }
}

/// Global application context.
#[derive(Clone)]
pub struct AppContext {
    pub client: TokocryptoClient,
    pub format: OutputFormat,
    pub verbose: bool,
    pub yes: bool,
}

#[derive(Parser, Debug)]
#[command(
    name = "tokocrypto",
    version,
    about = "Unofficial CLI for the Tokocrypto cryptocurrency exchange",
    long_about = "Trade, track markets, and manage your account on Tokocrypto — from your terminal.\n\n\
                  Built with Rust for maximum performance and safety.\n\
                  API docs: https://www.tokocrypto.com"
)]
pub struct Cli {
    /// Output format: table or json
    #[arg(short, long, default_value = "table", global = true)]
    pub output: OutputFormat,

    /// API key (overrides config and env var)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret (overrides config and env var)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Skip confirmation prompts for destructive operations
    #[arg(long, alias = "force", global = true)]
    pub yes: bool,

    /// Override API host URL
    #[arg(long, global = true)]
    pub host: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    // === Public Market Commands (originally nested under Market) ===
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
        pair: Option<String>,

        /// Query for multiple symbols (comma separated)
        #[arg(long)]
        pairs: Option<String>,

        /// Filter by symbol status (TRADING, HALT, BREAK)
        #[arg(long)]
        status: Option<String>,
    },

    /// Get order book depth
    Orderbook {
        /// Trading pair symbol (e.g., BTC_USDT or TKO_BIDR)
        pair: String,

        /// Limit number of price levels (default: 100, max: 5000)
        #[arg(short, long, default_value = "100")]
        count: u32,

        /// Manual symbol type override (1 - Main, 2 - Next, 3 - Nextme)
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Get recent trades
    Trades {
        /// Trading pair symbol
        pair: String,

        /// Start from this trade ID
        #[arg(long, alias = "from-id")]
        since: Option<i64>,

        /// Limit number of trades (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,

        /// Manual symbol type override
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Get compressed/aggregate trades list
    AggTrades {
        /// Trading pair symbol
        pair: String,

        /// Trade ID to fetch from
        #[arg(long, alias = "from-id")]
        since: Option<i64>,

        /// Start time in milliseconds
        #[arg(long)]
        start_time: Option<i64>,

        /// End time in milliseconds
        #[arg(long)]
        end_time: Option<i64>,

        /// Limit number of trades (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,

        /// Manual symbol type override
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    /// Get kline/candlestick data bars
    Klines {
        /// Trading pair symbol
        pair: String,

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
        count: u32,

        /// Manual symbol type override
        #[arg(long)]
        symbol_type: Option<u32>,
    },

    // === Account & Balance Commands (originally nested under Account) ===
    /// Get current account details (commissions, permissions)
    AccountInfo,

    /// Get non-zero asset balances
    Balance,

    /// Get details of a specific asset
    Assets {
        /// Asset name (e.g., ADA, BTC, USDT)
        asset: String,
    },

    /// Get your trade history for a symbol
    TradesHistory {
        /// Trading pair symbol (e.g., BTC_USDT)
        pair: String,

        /// Start from this trade ID
        #[arg(long, alias = "from-id")]
        since: Option<i64>,

        /// Limit number of trades (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    // === Trading Operations (originally nested under Trade) ===
    /// Place and manage orders
    #[command(subcommand)]
    Order(trade::OrderCommand),

    // === Funding / Withdrawal Operations (originally nested under Funding) ===
    /// Withdraw crypto to an external address
    Withdraw {
        /// Asset to withdraw (e.g., BTC, USDT, TKO)
        #[arg(long)]
        asset: String,

        /// Amount to withdraw
        #[arg(long)]
        volume: String,

        /// Target wallet address
        #[arg(long)]
        address: String,

        /// Address description tag (default: "Withdrawal Address")
        #[arg(long, default_value = "Withdrawal Address")]
        tag: String,

        /// Memo or Tag for coins that require it (e.g. XRP, EOS)
        #[arg(long)]
        memo: Option<String>,

        /// Network to withdraw on (e.g. BSC, ETH, TRX)
        #[arg(long)]
        network: Option<String>,
    },

    /// Manage cryptocurrency deposits
    #[command(subcommand)]
    Deposit(DepositSubcommand),

    /// Manage cryptocurrency withdrawals
    #[command(subcommand)]
    Withdrawal(WithdrawalSubcommand),

    // === WS, Paper, Auth, Shell, Mcp ===
    /// WebSocket real-time data streams
    #[command(subcommand)]
    Ws(websocket::WebSocketCommand),

    /// Paper trading (simulated)
    #[command(subcommand)]
    Paper(paper::PaperCommand),

    /// API credential management
    #[command(subcommand)]
    Auth(auth_cmds::AuthCommand),

    /// Start interactive REPL shell
    Shell,

    /// Run as an MCP (Model Context Protocol) server
    Mcp {
        /// Allow dangerous commands (trade, funding) (ignored for now, present for compatibility)
        #[arg(long)]
        allow_dangerous: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DepositSubcommand {
    /// List crypto deposit history
    Status {
        /// Filter by asset (e.g., USDT)
        #[arg(long)]
        asset: Option<String>,

        /// Filter by status (0: Pending, 6: Success, 1: Failed)
        #[arg(long)]
        status: Option<i32>,

        /// Limit number of records (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// Get deposit address for a specific coin
    Addresses {
        /// Coin name (e.g., USDT, BTC)
        asset: String,

        /// Network type (e.g. ETH, BSC, TRX)
        #[arg(long)]
        network: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum WithdrawalSubcommand {
    /// List crypto withdraw history
    Status {
        /// Filter by asset
        #[arg(long)]
        asset: Option<String>,

        /// Filter by status (0: Email Sent, 1: Cancelled, 2: Awaiting Approval, 3: Rejected, 4: Processing, 5: Failure, 6: Completed)
        #[arg(long)]
        status: Option<i32>,

        /// Limit number of records (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },
}

/// Dispatch all non-shell commands to their executors.
pub async fn dispatch_non_shell(
    ctx: &AppContext,
    command: Command,
) -> Result<CommandOutput, TokocryptoError> {
    match command {
        // === Public Market Commands ===
        Command::Ping => market::MarketCommand::Ping.execute(ctx).await,
        Command::ServerTime => market::MarketCommand::ServerTime.execute(ctx).await,
        Command::Symbols => market::MarketCommand::Symbols.execute(ctx).await,
        Command::ExecutionRules {
            pair,
            pairs,
            status,
        } => {
            market::MarketCommand::ExecutionRules {
                symbol: pair.map(|pair| normalize_pair(&pair)),
                symbols: pairs.map(|pairs| normalize_pair_list(&pairs)),
                status,
            }
            .execute(ctx)
            .await
        }
        Command::Orderbook {
            pair,
            count,
            symbol_type,
        } => {
            market::MarketCommand::Depth {
                symbol: normalize_pair(&pair),
                limit: count,
                symbol_type,
            }
            .execute(ctx)
            .await
        }
        Command::Trades {
            pair,
            since,
            count,
            symbol_type,
        } => {
            market::MarketCommand::Trades {
                symbol: normalize_pair(&pair),
                from_id: since,
                limit: count,
                symbol_type,
            }
            .execute(ctx)
            .await
        }
        Command::AggTrades {
            pair,
            since,
            start_time,
            end_time,
            count,
            symbol_type,
        } => {
            market::MarketCommand::AggTrades {
                symbol: normalize_pair(&pair),
                from_id: since,
                start_time,
                end_time,
                limit: count,
                symbol_type,
            }
            .execute(ctx)
            .await
        }
        Command::Klines {
            pair,
            interval,
            start_time,
            end_time,
            count,
            symbol_type,
        } => {
            market::MarketCommand::Klines {
                symbol: normalize_pair(&pair),
                interval,
                start_time,
                end_time,
                limit: count,
                symbol_type,
            }
            .execute(ctx)
            .await
        }

        // === Account & Balance Commands ===
        Command::AccountInfo => account::AccountCommand::Info.execute(ctx).await,
        Command::Balance => account::AccountCommand::Balance.execute(ctx).await,
        Command::Assets { asset } => account::AccountCommand::Assets { asset }.execute(ctx).await,
        Command::TradesHistory { pair, since, count } => {
            account::AccountCommand::Trades {
                symbol: normalize_pair(&pair),
                from_id: since,
                limit: count,
            }
            .execute(ctx)
            .await
        }

        // === Trading Operations ===
        Command::Order(cmd) => cmd.execute(ctx).await,

        // === Funding / Withdrawal Operations ===
        Command::Withdraw {
            asset,
            volume,
            address,
            tag,
            memo,
            network,
        } => {
            funding::FundingCommand::Withdraw {
                coin: asset,
                amount: volume,
                address,
                tag,
                memo,
                network,
            }
            .execute(ctx)
            .await
        }
        Command::Deposit(sub) => {
            let funding_cmd = match sub {
                DepositSubcommand::Status {
                    asset,
                    status,
                    count,
                } => funding::FundingCommand::DepositHistory {
                    coin: asset,
                    status,
                    limit: count,
                },
                DepositSubcommand::Addresses { asset, network } => {
                    funding::FundingCommand::DepositAddress {
                        coin: asset,
                        network,
                    }
                }
            };
            funding_cmd.execute(ctx).await
        }
        Command::Withdrawal(sub) => {
            let funding_cmd = match sub {
                WithdrawalSubcommand::Status {
                    asset,
                    status,
                    count,
                } => funding::FundingCommand::WithdrawHistory {
                    coin: asset,
                    status,
                    limit: count,
                },
            };
            funding_cmd.execute(ctx).await
        }

        // === WS, Paper, Auth, Shell, Mcp ===
        Command::Ws(cmd) => cmd.execute(ctx).await,
        Command::Paper(cmd) => cmd.execute(ctx).await,
        Command::Auth(cmd) => cmd.execute(ctx).await,
        Command::Shell => Err(TokocryptoError::Config(
            "Shell command is not supported in this context".to_string(),
        )),
        Command::Mcp { .. } => Err(TokocryptoError::Config(
            "MCP server must be started from the main entry point".to_string(),
        )),
    }
}

/// Dispatch the parsed command to its executor.
pub async fn dispatch(
    ctx: &AppContext,
    command: Command,
) -> Result<CommandOutput, TokocryptoError> {
    match command {
        Command::Shell => {
            utility::run_shell(ctx).await?;
            Ok(CommandOutput::new(serde_json::json!({}), "Shell").with_format(ctx.format))
        }
        other => dispatch_non_shell(ctx, other).await,
    }
}
