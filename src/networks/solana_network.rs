use reqwest::blocking::Client;
use anyhow::Result;
use serde_json;
use ed25519_dalek::{SigningKey, Signer};
use bs58;
use crate::types::{Transaction, Direction, Status};
use crate::making_tx::Network;

pub fn rpc_call(method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let client = Client::new();
    let url = "https://api.devnet.solana.com";

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let resp = client.post(url).json(&body).send()?;
    let json: serde_json::Value = resp.json()?;
    Ok(json)
}

pub fn get_sol_balance(address: &str) -> Result<f64> {
    let address = address.trim();
    let resp = rpc_call("getBalance", serde_json::json!([address]))?;

    let lamports = resp["result"]["value"]
        .as_u64()
        .ok_or(anyhow::anyhow!("Failed to parse balance"))?;

    Ok(lamports as f64 / 1_000_000_000.0)
}

pub fn get_latest_blockhash() -> Result<String> {
    let resp = rpc_call("getLatestBlockhash", serde_json::json!([]))?;

    let blockhash = resp["result"]["value"]["blockhash"]
        .as_str()
        .ok_or(anyhow::anyhow!("Failed to parse blockhash"))?
        .to_string();

    Ok(blockhash)
}

pub fn send_sol(signing_key: &SigningKey, recipient: &str, amount_sol: f64) -> Result<String> {
    // Step 1 — get latest blockhash
    let blockhash_str = get_latest_blockhash()?;
    let blockhash_bytes = bs58::decode(&blockhash_str)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Failed to decode blockhash: {}", e))?;

    // Step 2 — get sender public key
    let sender_pubkey = signing_key.verifying_key();
    let sender_bytes = sender_pubkey.as_bytes();

    // Step 3 — decode recipient address
    let recipient_bytes = bs58::decode(recipient.trim())
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid recipient address: {}", e))?;

    // Step 4 — convert SOL to lamports
    let lamports = (amount_sol * 1_000_000_000.0) as u64;

    // Step 5 — build the transaction message
    // Solana transaction format:
    // [num_signatures][num_readonly_signed][num_readonly_unsigned]
    // [num_accounts][accounts...][blockhash][num_instructions][instructions...]
    let mut message: Vec<u8> = vec![
        1,    // num required signatures
        0,    // num readonly signed accounts
        1,    // num readonly unsigned accounts (system program)
    ];

    // accounts: sender, recipient, system program
    let system_program = vec![0u8; 32]; // system program is all zeros
    message.push(3); // number of accounts
    message.extend_from_slice(sender_bytes);
    message.extend_from_slice(&recipient_bytes);
    message.extend_from_slice(&system_program);

    // recent blockhash
    message.extend_from_slice(&blockhash_bytes);

    // instruction: system transfer
    message.push(1); // number of instructions
    message.push(2); // program id index (system program = index 2)
    message.push(2); // number of accounts in instruction
    message.push(0); // sender account index
    message.push(1); // recipient account index

    // instruction data: transfer (2) + lamports (8 bytes little endian)
    let mut instruction_data = vec![2u8, 0, 0, 0]; // transfer instruction
    instruction_data.extend_from_slice(&lamports.to_le_bytes());
    message.push(instruction_data.len() as u8);
    message.extend_from_slice(&instruction_data);

    // Step 6 — sign the message
    let signature = signing_key.sign(&message);
    let signature_bytes = signature.to_bytes();

    // Step 7 — build full transaction
    // [num_signatures][signature][message]
    let mut transaction: Vec<u8> = vec![1]; // 1 signature
    transaction.extend_from_slice(&signature_bytes);
    transaction.extend_from_slice(&message);

    // Step 8 — encode as base64 and send
    let tx_base64 = base64_encode(&transaction);

    let resp = rpc_call("sendTransaction", serde_json::json!([
        tx_base64,
        { "encoding": "base64" }
    ]))?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("Transaction failed: {}", err);
    }

    let signature = resp["result"]
        .as_str()
        .ok_or(anyhow::anyhow!("Failed to get transaction signature"))?
        .to_string();

    Ok(signature)
}

// Simple base64 encoder (no extra crate needed)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };

        result.push(CHARS[((b0 >> 2) & 0x3F) as usize] as char);
        result.push(CHARS[(((b0 << 4) | (b1 >> 4)) & 0x3F) as usize] as char);
        result.push(if i + 1 < data.len() { CHARS[(((b1 << 2) | (b2 >> 6)) & 0x3F) as usize] as char } else { '=' });
        result.push(if i + 2 < data.len() { CHARS[(b2 & 0x3F) as usize] as char } else { '=' });

        i += 3;
    }

    result
}

pub fn map_solana_tx(result: &serde_json::Value, my_address: &str) -> Option<Transaction> {
    let meta = &result["meta"];
    let message = &result["transaction"]["message"];

    let account_keys = message["accountKeys"].as_array()?;
    let address_from = account_keys[0].as_str()?.to_string();
    let address_to = account_keys[1].as_str()?.to_string();

    let pre_balances = meta["preBalances"].as_array()?;
    let post_balances = meta["postBalances"].as_array()?;

    let receiver_pre = pre_balances[1].as_u64()?;
    let receiver_post = post_balances[1].as_u64()?;
    let amount = (receiver_post - receiver_pre) as f64 / 1_000_000_000.0;
    let fee = meta["fee"].as_u64()? as f64 / 1_000_000_000.0;

    let direction = if my_address == address_from {
        Direction::Sent
    } else if my_address == address_to {
        Direction::Receive
    } else {
        return None;
    };

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
