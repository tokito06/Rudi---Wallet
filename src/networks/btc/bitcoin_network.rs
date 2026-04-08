use std::ptr::addr_of;

use bitcoin::address;
use reqwest::{Url, blocking::Client};
use anyhow::Result;
use serde::{Deserialize};


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