use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use crate::helpers::making_tx::Network;
use crate::networks::solana_network;
use crate::networks::btc::bitcoin_network;
use crate::networks::ethereum_network;
use crate::helpers::types::Transaction;

pub struct History {
    history: Arc<Mutex<Vec<Transaction>>>
}

fn history_path() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
    let home = std::fs::canonicalize(&home).unwrap_or(home);
    home.join(".crypto-wallet-history.dat")
}

impl History {
    pub async fn new(sol_address: &str, btc_address: &str, eth_address: &str) -> Result<History> {
        let mut history = match History::load_history() {
            Ok(h) => h,
            Err(_) => History { history: Arc::new(Mutex::new(vec![])) },
        };
        history.append(sol_address, btc_address, eth_address).await?;
        Ok(history)
    }

    pub fn store_history(&self) -> Result<()> {
        let path = history_path();
        let history = self.history
            .lock()
            .map_err(|_| anyhow::anyhow!("History mutex was poisoned"))?;
        let json = serde_json::to_string_pretty(&*history)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn load_history() -> Result<History> {
        let path = history_path();

        if !path.exists() {
            anyhow::bail!("No history exists yet");
        }

        let json = fs::read_to_string(&path)?;
        let transactions: Vec<Transaction> = serde_json::from_str(&json)
            .context("History file is corrupted")?;

        Ok(History { history: Arc::new(Mutex::new(transactions)) })
    }

    fn last_entry_network(&self, network: &Network) -> Option<String> {
        self.history
            .lock()
            .ok()?
            .iter()
            .rev()
            .find(|tx| tx.network == *network)
            .map(|tx| tx.txid.clone())
    }

    pub async fn append(&mut self, sol_address: &str, btc_address: &str, eth_address: &str) -> Result<()> {
        let since_btc = self.last_entry_network(&Network::Btc);
        let since_sol = self.last_entry_network(&Network::Sol);
        let since_eth = self.last_entry_network(&Network::Eth);

        let btc_address = btc_address.to_string();
        let sol_address = sol_address.to_string();
        let eth_address = eth_address.to_string();

        let (btc_txs, sol_txs, eth_txs) = tokio::join!(
            bitcoin_network::fetch_btc_history(&btc_address, since_btc),
            solana_network::fetch_sol_history(&sol_address, since_sol),
            ethereum_network::fetch_eth_history(&eth_address, since_eth),
        );

        let mut all_txs: Vec<Transaction> = vec![];
        all_txs.extend(btc_txs?);
        all_txs.extend(sol_txs?);
        all_txs.extend(eth_txs?);
        all_txs.reverse();

        self.history
            .lock()
            .map_err(|_| anyhow::anyhow!("History mutex was poisoned"))?
            .extend(all_txs);

        self.store_history()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::making_tx::Network;
    use crate::helpers::types::{Transaction, Direction, Status};
    use tempfile::TempDir;
    use serial_test::serial;

    fn setup(dir: &TempDir) {
        let path = dir.path().to_path_buf();
        std::fs::create_dir_all(&path).unwrap();
        std::env::set_var("HOME", path);
    }

    fn mock_tx(network: Network, txid: &str) -> Transaction {
        Transaction {
            txid: txid.to_string(),
            network,
            direction: Direction::Received,
            status: Status::Success,
            timestamp: "1000000".to_string(),
            amount: 0.1,
            address_from: "addr_from".to_string(),
            address_to: "addr_to".to_string(),
            fee: 0.001,
        }
    }

    fn make_history(txs: Vec<Transaction>) -> History {
        History {
            history: Arc::new(Mutex::new(txs)),
        }
    }

    #[test]
    #[serial]
    fn test_history_path_ends_with_dat() {
        let path = history_path();
        assert!(path.to_str().unwrap().ends_with(".crypto-wallet-history.dat"));
    }

    #[test]
    #[serial]
    fn test_load_history_fails_when_no_file() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        assert!(History::load_history().is_err());
    }

    #[test]
    #[serial]
    fn test_load_history_fails_on_corrupted_file() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        std::fs::write(history_path(), "not valid json").unwrap();
        assert!(History::load_history().is_err());
    }

    #[test]
    #[serial]
    fn test_store_and_load_history() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        let txs = vec![
            mock_tx(Network::Btc, "btc-tx-1"),
            mock_tx(Network::Sol, "sol-tx-1"),
        ];
        let history = make_history(txs);
        history.store_history().unwrap();

        let loaded = History::load_history().unwrap();
        let loaded_txs = loaded.history.lock().unwrap();
        assert_eq!(loaded_txs.len(), 2);
        assert_eq!(loaded_txs[0].txid, "btc-tx-1");
        assert_eq!(loaded_txs[1].txid, "sol-tx-1");
    }

    #[test]
    #[serial]
    fn test_store_history_empty() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        let history = make_history(vec![]);
        assert!(history.store_history().is_ok());
    }

    #[test]
    #[serial]
    fn test_last_entry_network_returns_last_txid() {
        let history = make_history(vec![
            mock_tx(Network::Btc, "btc-tx-1"),
            mock_tx(Network::Btc, "btc-tx-2"),
        ]);
        assert_eq!(
            history.last_entry_network(&Network::Btc),
            Some("btc-tx-2".to_string())
        );
    }

    #[test]
    #[serial]
    fn test_last_entry_network_returns_none_when_empty() {
        let history = make_history(vec![]);
        assert_eq!(history.last_entry_network(&Network::Btc), None);
    }

    #[test]
    #[serial]
    fn test_last_entry_network_returns_none_for_missing_network() {
        let history = make_history(vec![
            mock_tx(Network::Btc, "btc-tx-1"),
        ]);
        assert_eq!(history.last_entry_network(&Network::Sol), None);
    }

    #[test]
    #[serial]
    fn test_last_entry_network_ignores_other_networks() {
        let history = make_history(vec![
            mock_tx(Network::Btc, "btc-tx-1"),
            mock_tx(Network::Sol, "sol-tx-1"),
            mock_tx(Network::Eth, "eth-tx-1"),
        ]);
        assert_eq!(
            history.last_entry_network(&Network::Sol),
            Some("sol-tx-1".to_string())
        );
    }

    #[test]
    #[serial]
    fn test_store_history_poisoned_mutex() {
        let history = make_history(vec![]);
        let arc = Arc::clone(&history.history);
        let _ = std::panic::catch_unwind(|| {
            let _guard = arc.lock().unwrap();
            panic!("poison the mutex");
        });
        assert!(history.store_history().is_err());
    }

    #[test]
    #[serial]
    fn test_store_history_persists_multiple_networks() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        let txs = vec![
            mock_tx(Network::Btc, "btc-tx-1"),
            mock_tx(Network::Sol, "sol-tx-1"),
            mock_tx(Network::Eth, "eth-tx-1"),
        ];
        let history = make_history(txs);
        history.store_history().unwrap();

        let loaded = History::load_history().unwrap();
        let loaded_txs = loaded.history.lock().unwrap();
        assert_eq!(loaded_txs.len(), 3);
    }

    #[test]
    #[serial]
    fn test_load_history_preserves_transaction_fields() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        let tx = mock_tx(Network::Btc, "btc-tx-1");
        let history = make_history(vec![tx]);
        history.store_history().unwrap();

        let loaded = History::load_history().unwrap();
        let loaded_txs = loaded.history.lock().unwrap();
        let loaded_tx = &loaded_txs[0];
        assert_eq!(loaded_tx.txid, "btc-tx-1");
        assert_eq!(loaded_tx.amount, 0.1);
        assert_eq!(loaded_tx.fee, 0.001);
        assert_eq!(loaded_tx.address_from, "addr_from");
        assert_eq!(loaded_tx.address_to, "addr_to");
        assert_eq!(loaded_tx.timestamp, "1000000");
    }
}