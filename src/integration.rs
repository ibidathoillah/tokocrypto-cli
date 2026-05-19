//! Stable integration surface for consumers that embed this crate as a
//! submodule or path dependency.

pub use crate::client::TokocryptoClient;
pub use crate::config::{Config, Credentials, DEFAULT_HOST, DEFAULT_SITE_HOST};
pub use crate::errors::TokocryptoError;
pub use crate::output::{CommandOutput, OutputFormat};
pub use crate::{
    dispatch, dispatch_non_shell, normalize_pair, normalize_pair_list, normalize_pair_ws,
    AppContext, Cli, Command,
};

/// Convenience imports for external consumers.
pub mod prelude {
    pub use super::{
        dispatch, dispatch_non_shell, normalize_pair, normalize_pair_list, normalize_pair_ws,
        AppContext, Cli, Command, CommandOutput, Config, Credentials, DEFAULT_HOST,
        DEFAULT_SITE_HOST, OutputFormat, TokocryptoClient, TokocryptoError,
    };
}
