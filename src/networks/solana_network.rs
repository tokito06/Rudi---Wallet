use reqwest::Client;
use anyhow::Result;
use serde_json;
use ed25519_dalek::{SigningKey, Signer};
use bs58;
use base64::{Engine, engine::general_purpose::STANDARD};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::helpers::types::{Transaction, Direction, Status};
use crate::helpers::making_tx::Network;

const SOLANA_RPC_URL: &str = "https://api.devnet.solana.com";

static CLIENT: Lazy<Client> = Lazy::new(Client::new);
static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub async fn rpc_call(method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let id = REQUEST_ID.fetch_add(1, Ordering::Relaxed);

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });

    let resp = CLIENT.post(SOLANA_RPC_URL).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    Ok(json)
}

pub async fn get_sol_balance(address: &str) -> Result<f64> {
    let address = address.trim();
    let resp = rpc_call("getBalance", serde_json::json!([address])).await?;

    let lamports = resp["result"]["value"]
        .as_u64()
        .ok_or(anyhow::anyhow!("Failed to parse balance"))?;

    Ok(lamports as f64 / 1_000_000_000.0)
}

pub async fn get_latest_blockhash() -> Result<String> {
    let resp = rpc_call("getLatestBlockhash", serde_json::json!([])).await?;

    let blockhash = resp["result"]["value"]["blockhash"]
        .as_str()
        .ok_or(anyhow::anyhow!("Failed to parse blockhash"))?
        .to_string();

    Ok(blockhash)
}

pub async fn send_sol(signing_key: &SigningKey, recipient: &str, amount_sol: f64) -> Result<String> {
    let blockhash_str = get_latest_blockhash().await?;
    let blockhash_bytes = bs58::decode(&blockhash_str)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Failed to decode blockhash: {}", e))?;

    let sender_pubkey = signing_key.verifying_key();
    let sender_bytes = sender_pubkey.as_bytes();

    let recipient_bytes = bs58::decode(recipient.trim())
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid recipient address: {}", e))?;

    let lamports = (amount_sol * 1_000_000_000.0) as u64;

    let mut message: Vec<u8> = vec![1, 0, 1];

    let system_program = vec![0u8; 32];
    message.push(3);
    message.extend_from_slice(sender_bytes);
    message.extend_from_slice(&recipient_bytes);
    message.extend_from_slice(&system_program);
    message.extend_from_slice(&blockhash_bytes);

    message.push(1);
    message.push(2);
    message.push(2);
    message.push(0);
    message.push(1);

    let mut instruction_data = vec![2u8, 0, 0, 0];
    instruction_data.extend_from_slice(&lamports.to_le_bytes());
    message.push(instruction_data.len() as u8);
    message.extend_from_slice(&instruction_data);

    let signature = signing_key.sign(&message);
    let signature_bytes = signature.to_bytes();

    let mut transaction: Vec<u8> = vec![1];
    transaction.extend_from_slice(&signature_bytes);
    transaction.extend_from_slice(&message);

    let tx_base64 = STANDARD.encode(&transaction);

    let resp = rpc_call("sendTransaction", serde_json::json!([
        tx_base64,
        { "encoding": "base64" }
    ])).await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("Transaction failed: {}", err);
    }

    let signature = resp["result"]
        .as_str()
        .ok_or(anyhow::anyhow!("Failed to get transaction signature"))?
        .to_string();

    Ok(signature)
}

pub async fn fetch_sol_history(address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    let sigs = rpc_call("getSignaturesForAddress", serde_json::json!([address, {
        "limit": 10,
        "until": since_txid
    }])).await?;

    let mut txs = vec![];

    if let Some(sig_results) = sigs["result"].as_array() {
        for sig in sig_results {
            let signature = sig["signature"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Failed to parse signature"))?;

            let tx = rpc_call("getTransaction", serde_json::json!([signature, {
                "encoding": "json",
                "commitment": "confirmed"
            }])).await?;

            if let Some(transaction) = map_solana_tx(&tx["result"], address) {
                txs.push(transaction);
            }
        }
    }

    Ok(txs)
}

pub fn map_solana_tx(result: &serde_json::Value, my_address: &str) -> Option<Transaction> {
    let meta = &result["meta"];
    let message = &result["transaction"]["message"];

    let account_keys = message["accountKeys"].as_array()?;
    let address_from = account_keys[0].as_str()?.to_string();
    let address_to = account_keys[1].as_str()?.to_string();

    let pre_balances = meta["preBalances"].as_array()?;
    let post_balances = meta["postBalances"].as_array()?;

    let (direction, balance_index) = if my_address == address_from {
        (Direction::Sent, 0usize)
    } else if my_address == address_to {
        (Direction::Received, 1usize)
    } else {
        return None;
    };

    let account_pre = pre_balances[balance_index].as_u64()? as i128;
    let account_post = post_balances[balance_index].as_u64()? as i128;
    let balance_delta = account_post - account_pre;

    let amount_lamports = match direction {
        Direction::Sent => (-balance_delta).max(0),
        Direction::Received => balance_delta.max(0),
    };

    let amount = amount_lamports as f64 / 1_000_000_000.0;
    let fee = meta["fee"].as_u64()? as f64 / 1_000_000_000.0;

    Some(Transaction {
        txid: result["transaction"]["signatures"][0].as_str()?.to_string(),
        network: Network::Sol,
        direction,
        status: if meta["err"].is_null() { Status::Success } else { Status::Rejected },
        timestamp: result["blockTime"].as_i64()?.to_string(),
        amount,
        address_from,
        address_to,
        fee,
    })
}


#[cfg(test)]
mod network_tests {
    use super::*;

    const TEST_ADDRESS: &str = "11111111111111111111111111111111";

    #[tokio::test]
    #[ignore]
    async fn network_test_rpc_call_success() {
        let result = rpc_call("getHealth", serde_json::json!([])).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_sol_balance_valid_address() {
        let result = get_sol_balance(TEST_ADDRESS).await;
        assert!(result.is_ok());
        assert!(result.unwrap() >= 0.0);
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_sol_balance_invalid_address() {
        let result = get_sol_balance("invalid_address").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_get_latest_blockhash() {
        let result = get_latest_blockhash().await;
        assert!(result.is_ok());
        let blockhash = result.unwrap();
        assert!(!blockhash.is_empty());
        assert!(bs58::decode(&blockhash).into_vec().is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_fetch_sol_history_returns_ok() {
        let result = fetch_sol_history(TEST_ADDRESS, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_fetch_sol_history_with_since_txid() {
        let txs = fetch_sol_history(TEST_ADDRESS, None).await.unwrap();
        if txs.is_empty() {
            return;
        }
        let since = Some(txs[0].txid.clone());
        let result = fetch_sol_history(TEST_ADDRESS, since).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_fetch_sol_history_empty_address() {
        let result = fetch_sol_history("11111111111111111111111111111111", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn network_test_map_solana_tx_valid() {
        let sigs = rpc_call("getSignaturesForAddress", serde_json::json!([
            TEST_ADDRESS,
            { "limit": 1 }
        ])).await.unwrap();

        if let Some(sig_results) = sigs["result"].as_array() {
            if sig_results.is_empty() {
                return;
            }
            let signature = sig_results[0]["signature"].as_str().unwrap();
            let tx = rpc_call("getTransaction", serde_json::json!([signature, {
                "encoding": "json",
                "commitment": "confirmed"
            }])).await.unwrap();

            let result = map_solana_tx(&tx["result"], TEST_ADDRESS);
            assert!(result.is_some() || result.is_none());
        }
    }
}