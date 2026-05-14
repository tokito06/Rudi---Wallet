use reqwest::Client;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::Deserialize;
use crate::helpers::types::{Transaction, Direction, Status};
use crate::helpers::making_tx::Network;

const BTC_API_URL: &str = "https://blockstream.info/testnet/api";

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

#[derive(Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
}

pub async fn get_btc_balance(address: &str) -> Result<f64> {
    let url = format!("{}/address/{}/utxo", BTC_API_URL, address);
    let utxos: Vec<Utxo> = CLIENT.get(&url).send().await?.json().await?;
    let total: u64 = utxos.iter().map(|u| u.value).sum();
    Ok(total as f64 / 100_000_000.0)
}

pub async fn get_utxos(address: &str) -> Result<Vec<Utxo>> {
    let url = format!("{}/address/{}/utxo", BTC_API_URL, address);
    let utxos: Vec<Utxo> = CLIENT.get(&url).send().await?.json().await?;
    Ok(utxos)
}

pub async fn broadcast_tx(tx_hex: &str) -> Result<String> {
    let url = format!("{}/tx", BTC_API_URL);
    let response = CLIENT.post(&url).body(tx_hex.to_string()).send().await?;
    let txid = response.text().await?;
    Ok(txid)
}

pub async fn fetch_btc_history(address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    let url = format!("{}/address/{}/txs", BTC_API_URL, address);
    let raw_txs: serde_json::Value = CLIENT.get(&url).send().await?.json().await?;
    let mut txs = vec![];

    if let Some(raw_txs) = raw_txs.as_array() {
        for tx in raw_txs {
            if tx["txid"].as_str() == since_txid.as_deref() {
                break;
            }
            if let Some(transaction) = map_bitcoin_tx(tx, address) {
                txs.push(transaction);
            }
        }
    }

    Ok(txs)
}

pub fn map_bitcoin_tx(result: &serde_json::Value, my_address: &str) -> Option<Transaction> {
    let vin = result["vin"].as_array()?;
    let vout = result["vout"].as_array()?;

    let address_from = vin[0]["prevout"]["scriptpubkey_address"]
        .as_str()?
        .to_string();

    let is_sender = address_from == my_address;

    let (address_to, amount_sat) = if is_sender {
        let out = vout.iter().find(|o| {
            o["scriptpubkey_address"].as_str().unwrap_or("") != my_address
        })?;
        let addr = out["scriptpubkey_address"].as_str()?.to_string();
        let value = out["value"].as_u64()?;
        (addr, value)
    } else {
        let amount: u64 = vout.iter()
            .filter(|o| o["scriptpubkey_address"].as_str().unwrap_or("") == my_address)
            .filter_map(|o| o["value"].as_u64())
            .sum();
        if amount == 0 {
            return None;
        }
        (my_address.to_string(), amount)
    };

    let direction = if is_sender { Direction::Sent } else { Direction::Received };

    Some(Transaction {
        txid: result["txid"].as_str()?.to_string(),
        network: Network::Btc,
        direction,
        status: if result["status"]["confirmed"].as_bool().unwrap_or(false) {
            Status::Success
        } else {
            Status::Pending
        },
        timestamp: result["status"]["block_time"].as_i64()?.to_string(),
        amount: amount_sat as f64 / 100_000_000.0,
        address_from,
        address_to,
        fee: result["fee"].as_u64()? as f64 / 100_000_000.0,
    })
}


#[cfg(test)]
mod network_tests {
    use super::*;

    const TEST_ADDRESS: &str = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";

    async fn fresh_client_get(url: &str) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let resp = client.get(url).send().await?;
        Ok(resp.json().await?)
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_btc_balance_valid_address() {
        let result = get_btc_balance(TEST_ADDRESS).await;
        assert!(result.is_ok());
        assert!(result.unwrap() >= 0.0);
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_btc_balance_invalid_address() {
        let result = get_btc_balance("invalid_address").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_utxos_valid_address() {
        let result = get_utxos(TEST_ADDRESS).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_utxos_returns_vec() {
        let utxos = get_utxos(TEST_ADDRESS).await.unwrap();
        assert!(utxos.len() >= 0);
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_fetch_btc_history_returns_ok() {
        let result = fetch_btc_history(TEST_ADDRESS, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_fetch_btc_history_with_since_txid() {
        let txs = fetch_btc_history(TEST_ADDRESS, None).await.unwrap();
        if txs.is_empty() {
            return;
        }
        let since = Some(txs[0].txid.clone());
        let result = fetch_btc_history(TEST_ADDRESS, since).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_fetch_btc_history_stops_at_since_txid() {
        let txs = fetch_btc_history(TEST_ADDRESS, None).await.unwrap();
        if txs.len() < 2 {
            return;
        }
        let since = Some(txs[0].txid.clone());
        let txs_since = fetch_btc_history(TEST_ADDRESS, since.clone()).await.unwrap();
        assert!(txs_since.len() < txs.len());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_map_bitcoin_tx_sent() {
        let tx = serde_json::json!({
            "txid": "abc123",
            "vin": [{ "prevout": { "scriptpubkey_address": TEST_ADDRESS } }],
            "vout": [
                { "scriptpubkey_address": TEST_ADDRESS, "value": 1000 },
                { "scriptpubkey_address": "tb1qother", "value": 5000 }
            ],
            "fee": 100,
            "status": { "confirmed": true, "block_time": 1000000 }
        });
        let result = map_bitcoin_tx(&tx, TEST_ADDRESS);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().direction, crate::helpers::types::Direction::Sent));
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_map_bitcoin_tx_received() {
        let tx = serde_json::json!({
            "txid": "abc123",
            "vin": [{ "prevout": { "scriptpubkey_address": "tb1qother" } }],
            "vout": [
                { "scriptpubkey_address": TEST_ADDRESS, "value": 5000 }
            ],
            "fee": 100,
            "status": { "confirmed": true, "block_time": 1000000 }
        });
        let result = map_bitcoin_tx(&tx, TEST_ADDRESS);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().direction, crate::helpers::types::Direction::Received));
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_map_bitcoin_tx_pending() {
        let tx = serde_json::json!({
            "txid": "abc123",
            "vin": [{ "prevout": { "scriptpubkey_address": "tb1qother" } }],
            "vout": [
                { "scriptpubkey_address": TEST_ADDRESS, "value": 5000 }
            ],
            "fee": 100,
            "status": { "confirmed": false, "block_time": 1000000 }
        });
        let result = map_bitcoin_tx(&tx, TEST_ADDRESS);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().status, crate::helpers::types::Status::Pending));
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_map_bitcoin_tx_missing_fields() {
        let tx = serde_json::json!({});
        let result = map_bitcoin_tx(&tx, TEST_ADDRESS);
        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_map_bitcoin_tx_zero_received_amount() {
        let tx = serde_json::json!({
            "txid": "abc123",
            "vin": [{ "prevout": { "scriptpubkey_address": "tb1qother" } }],
            "vout": [
                { "scriptpubkey_address": "tb1qanother", "value": 5000 }
            ],
            "fee": 100,
            "status": { "confirmed": true, "block_time": 1000000 }
        });
        let result = map_bitcoin_tx(&tx, TEST_ADDRESS);
        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_api_reachable() {
        let url = format!("{}/blocks/tip/height", BTC_API_URL);
        let result = fresh_client_get(&url).await;
        assert!(result.is_ok());
    }
}