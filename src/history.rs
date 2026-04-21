use serde_json::json;
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use crate::making_tx::Network;
use crate::networks::solana_network;
use crate::types::{Transaction};

pub struct History {
    history: Arc<Mutex<Vec<Transaction>>>
}

fn history_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".crypto-wallet-history.dat")
}

impl History {

    pub fn new(sol_address: &str, btc_address: &str, eth_address: &str) -> Result<History> {
        let mut history = match History::load_history() {
            Ok(h) => h,
            Err(_) => History { history: Arc::new(Mutex::new(vec![])) },
        };
        history.append(sol_address, btc_address, eth_address)?;
        Ok(history)
    }

    pub fn store_history(&self) -> Result<()> {
        let path = history_path();
        let file = &self.history;
        fs::write(&path, serde_json::to_string_pretty(&file)?)
            .with_context(|| format!("Could not write to {:?}", path))?;
        Ok(())
    }

    pub fn load_history() -> Result<History> {
        let path = history_path();

        if !path.exists() {
            anyhow::bail!("No history exists yet")
        }

        let json = fs::read_to_string(&path)?;
        let transactions: Vec<Transaction> = serde_json::from_str(&json)
            .context("History file is corrupted")?;

        Ok(History { history: Arc::new(Mutex::new(transactions)) })
    }

    fn last_entry_network(&self, network: &Network) -> Option<String> {
        for tx in self.history.lock().unwrap().iter().rev() {
            if tx.network == *network {
                return Some(tx.txid.clone());
            }
        }
        None
    }

    pub fn append(&mut self, sol_address: &str, btc_address: &str, eth_address: &str) -> Result<()> {
        for (network, address) in [
            (Network::Btc, btc_address),
            (Network::Sol, sol_address),
            (Network::Eth, eth_address),
        ] {
            let since_txid = self.last_entry_network(&network);
            let new_txs = fetch_network_history(&network, address, since_txid)?;
            self.history.lock().unwrap().extend(new_txs);
        }
        self.store_history()
    }
}

fn fetch_network_history(network: &Network, address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    match network {
        Network::Sol => {
            let sigs = solana_network::rpc_call("getSignaturesForAddress", json!([address, {
                "limit": 10,
                "until": since_txid
            }]))?;
            let mut txs = vec![];

            for sig in sigs["result"].as_array().unwrap_or(&vec![]) {
                let signature = sig["signature"].as_str().unwrap();
                let tx = solana_network::rpc_call("getTransaction", json!([signature, {
                    "encoding": "json",
                    "commitment": "confirmed"
                }]))?;

                if let Some(transaction) = solana_network::map_solana_tx(&tx["result"], address) {
                    txs.push(transaction);
                }
            }
            Ok(txs)
        }
        Network::Btc => Ok(vec![]),
        Network::Eth => Ok(vec![]),
    }
}