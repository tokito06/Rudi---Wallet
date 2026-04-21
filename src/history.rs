use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use crate::making_tx::Network;
use crate::networks::solana_network;
use crate::networks::btc::bitcoin_network;
use crate::networks::ethereum_network;
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
        let history = self.history
            .lock()
            .map_err(|_| anyhow::anyhow!("History mutex was poisoned"))?;
        let json = serde_json::to_string_pretty(&*history)?;
        fs::write(&path, json);
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
            let mut new_txs = fetch_network_history(&network, address, since_txid)?;
            new_txs.reverse();
            self.history.lock().unwrap().extend(new_txs);
        }
        self.store_history()
    }
}

fn fetch_network_history(network: &Network, address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    match network {
        Network::Sol => solana_network::fetch_sol_history(address, since_txid),
        Network::Btc => bitcoin_network::fetch_btc_history(address, since_txid),
        Network::Eth => ethereum_network::fetch_eth_history(address, since_txid),
    }
}