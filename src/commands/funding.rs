use clap::Subcommand;

use crate::errors::TokocryptoError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum FundingCommand {
    /// Withdraw crypto to an external address
    Withdraw {
        /// Asset to withdraw (e.g., BTC, USDT, TKO)
        #[arg(long)]
        coin: String,

        /// Amount to withdraw
        #[arg(long)]
        amount: String,

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

    /// List crypto withdraw history
    WithdrawHistory {
        /// Filter by coin
        #[arg(long)]
        coin: Option<String>,

        /// Filter by status (0: Email Sent, 1: Cancelled, 2: Awaiting Approval, 3: Rejected, 4: Processing, 5: Failure, 6: Completed)
        #[arg(long)]
        status: Option<i32>,

        /// Limit number of records (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },

    /// List crypto deposit history
    DepositHistory {
        /// Filter by coin
        #[arg(long)]
        coin: Option<String>,

        /// Filter by status (0: Pending, 6: Success, 1: Failed)
        #[arg(long)]
        status: Option<i32>,

        /// Limit number of records (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },

    /// Get deposit address for a specific coin
    DepositAddress {
        /// Coin name (e.g., USDT, BTC)
        coin: String,

        /// Network type (e.g. ETH, BSC, TRX)
        #[arg(long)]
        network: Option<String>,
    },
}

impl FundingCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, TokocryptoError> {
        let client = &ctx.client;

        let output = match self {
            Self::Withdraw {
                coin,
                amount,
                address,
                tag,
                memo,
                network,
            } => {
                let mut params = vec![
                    ("coin", coin.as_str()),
                    ("amount", amount.as_str()),
                    ("address", address.as_str()),
                    ("name", tag.as_str()),
                ];

                if let Some(ref m) = memo {
                    params.push(("memo", m.as_str()));
                }
                if let Some(ref net) = network {
                    params.push(("network", net.as_str()));
                }

                let result = client.post_signed("/open/v1/withdraws", &params).await?;
                CommandOutput::new(result, "Withdraw Result")
                    .with_addendum(format!("Withdrawal of {} {} submitted successfully", amount, coin))
            }

            Self::WithdrawHistory { coin, status, limit } => {
                let limit_str = limit.to_string();
                let status_str = status.map(|s| s.to_string());

                let mut params = vec![("limit", limit_str.as_str())];
                if let Some(ref c) = coin {
                    params.push(("coin", c.as_str()));
                }
                if let Some(ref s) = status_str {
                    params.push(("status", s.as_str()));
                }

                let result = client.get_signed("/open/v1/withdraws", &params).await?;
                CommandOutput::new(result, "Withdraw History")
            }

            Self::DepositHistory { coin, status, limit } => {
                let limit_str = limit.to_string();
                let status_str = status.map(|s| s.to_string());

                let mut params = vec![("limit", limit_str.as_str())];
                if let Some(ref c) = coin {
                    params.push(("coin", c.as_str()));
                }
                if let Some(ref s) = status_str {
                    params.push(("status", s.as_str()));
                }

                let result = client.get_signed("/open/v1/deposits", &params).await?;
                CommandOutput::new(result, "Deposit History")
            }

            Self::DepositAddress { coin, network } => {
                let coin_upper = coin.to_uppercase();
                let mut params = vec![
                    ("coin", coin_upper.as_str()),
                    ("asset", coin_upper.as_str()),
                ];

                if let Some(ref net) = network {
                    params.push(("network", net.as_str()));
                }

                let result = client.get_signed("/open/v1/deposits/address", &params).await?;
                CommandOutput::new(result, format!("Deposit Address — {}", coin_upper))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}
