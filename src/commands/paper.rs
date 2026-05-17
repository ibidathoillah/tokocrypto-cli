use clap::Subcommand;

use crate::errors::TokocryptoError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum PaperCommand {
    /// Show simulated paper trading balances
    Balance,
}

impl PaperCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, TokocryptoError> {
        match self {
            Self::Balance => {
                let data = serde_json::json!({
                    "balances": [
                        { "asset": "USDT", "free": "10000.00", "locked": "0.00" },
                        { "asset": "BIDR", "free": "100000000.00", "locked": "0.00" },
                        { "asset": "TKO", "free": "5000.00", "locked": "0.00" }
                    ]
                });
                Ok(CommandOutput::new(data, "Paper Balances")
                    .with_format(ctx.format)
                    .with_addendum("Paper trading is currently a simulated stub."))
            }
        }
    }
}
