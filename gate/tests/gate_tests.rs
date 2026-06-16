use tokocrypto_gate::{TokocryptoGate, ExchangeGate};

#[tokio::test]
async fn test_tokocrypto_gate_rest() {
    let gate = TokocryptoGate::new();
    
    // Test list_pairs
    let pairs = gate.list_pairs().await.expect("failed to list pairs");
    assert!(!pairs.is_empty(), "symbols list should not be empty");
    
    // Check if BTC/USDT or TKO/BIDR is in the list
    let btc_usdt = pairs.iter().find(|p| p.symbol == "BTC/USDT");
    assert!(btc_usdt.is_some(), "BTC/USDT should be in the list of pairs");
    let pair = btc_usdt.unwrap();
    assert_eq!(pair.base, "BTC");
    assert_eq!(pair.quote, "USDT");

    // Test last_price
    let tick = gate.last_price("BTC/USDT").await.expect("failed to get last price");
    assert_eq!(tick.symbol, "BTC/USDT");
    assert!(tick.price > 0.0, "BTC/USDT price should be positive");
}
