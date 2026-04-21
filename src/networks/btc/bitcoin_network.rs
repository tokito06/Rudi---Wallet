use reqwest::{blocking::Client};
use anyhow::Result;
use serde::{Deserialize};
use crate::helpers::types::{Transaction, Direction, Status};
use crate::helpers::making_tx::Network;

#[derive(Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
}


pub fn get_btc_balance(address: &str) -> Result<f64> {
    let client = Client::new();
    let url = format!("https://blockstream.info/testnet/api/address/{}/utxo", address);
    let utxos: Vec<Utxo> = client.get(url).send()?.json()?;

    let mut balance:f64 = 0.0;
    for utxo in utxos {
        balance += utxo.value as f64;
    }
    Ok(balance / 100_000_000.0)
}


pub fn get_utxos(address: &str)-> Result<Vec<Utxo>>{
    let client = Client::new();
    let url = format!("https://blockstream.info/testnet/api/address/{}/utxo", address);
    let utxos: Vec<Utxo> = client.get(url).send()?.json()?;
    Ok(utxos)
}


pub fn broadcast_tx(tx_hex: &str) -> Result<String> {
    let client = Client::new();
    let url = "https://blockstream.info/testnet/api/tx";
    
    let response = client
        .post(url)
        .body(tx_hex.to_string())
        .send()?;

    let txid = response.text()?;
    Ok(txid)
}

pub fn fetch_btc_history(address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    let client = Client::new();
    let url = format!("https://blockstream.info/testnet/api/address/{}/txs", address);
    let raw_txs: serde_json::Value = client.get(&url).send()?.json()?;
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
        if amount == 0 { return None; }
        (my_address.to_string(), amount)
    };

    let direction = if is_sender {
         Direction::Sent 
        } 
        else { 
            Direction::Received 
        };

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
