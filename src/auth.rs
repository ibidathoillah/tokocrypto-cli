use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generate an HMAC-SHA256 signature for the given message using the secret key.
pub fn sign(secret_key: &str, message: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can accept any key length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Append timestamp, recvWindow, and signature to a parameter/query string.
/// Returns the fully signed parameter string.
pub fn sign_params(secret_key: &str, query: &str, server_time: u64, recv_window: u64) -> String {
    let mut signed = if query.is_empty() {
        format!("recvWindow={}&timestamp={}", recv_window, server_time)
    } else {
        format!("{}&recvWindow={}&timestamp={}", query, recv_window, server_time)
    };

    let signature = sign(secret_key, &signed);
    signed.push_str(&format!("&signature={}", signature));
    signed
}
