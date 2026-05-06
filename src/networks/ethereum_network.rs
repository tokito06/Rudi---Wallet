use alloy::{
    providers::{Provider, ProviderBuilder},
    primitives::{Address, U256},
    network::{EthereumWallet, TransactionBuilder},
    rpc::types::TransactionRequest,
};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::Client;
use crate::helpers::types::{Transaction, Direction, Status};
use crate::helpers::making_tx::Network;

const ETH_RPC_URL: &str = "https://ethereum-sepolia-rpc.publicnode.com";
const ETH_HISTORY_URL: &str = "https://eth-sepolia.blockscout.com/api";
const ETH_CHAIN_ID: u64 = 11155111;
const WEI_PER_ETH: u128 = 1_000_000_000_000_000_000;

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

pub async fn get_provider() -> Result<impl Provider> {
    let provider = ProviderBuilder::new()
        .connect(ETH_RPC_URL)
        .await?;
    Ok(provider)
}

pub async fn get_eth_balance(address: &str) -> Result<f64> {
    let provider = get_provider().await?;
    let address: Address = address.parse()?;
    let balance = provider.get_balance(address).await?;
    Ok(balance.to::<u128>() as f64 / WEI_PER_ETH as f64)
}

pub async fn send_eth(signer: PrivateKeySigner, recipient: &str, amount_eth: f64) -> Result<String> {
    let to_address: Address = recipient.parse()?;
    let from_address = signer.address();
    let amount_wei = U256::from((amount_eth * WEI_PER_ETH as f64) as u128);

    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_chain_id(ETH_CHAIN_ID)
        .wallet(wallet)
        .connect(ETH_RPC_URL)
        .await?;

    let gas = provider.estimate_gas(
        TransactionRequest::default()
            .with_from(from_address)
            .with_to(to_address)
            .with_value(amount_wei)
    ).await?;

    let transaction = TransactionRequest::default()
        .with_to(to_address)
        .with_from(from_address)
        .with_value(amount_wei)
        .with_gas_limit(gas);

    let transaction_hash = provider.send_transaction(transaction).await?;

    Ok(transaction_hash.tx_hash().to_string())
}

pub async fn fetch_eth_history(address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    let url = format!(
        "{}?module=account&action=txlist&address={}&sort=desc",
        ETH_HISTORY_URL, address
    );

    let resp: serde_json::Value = CLIENT.get(&url).send().await?.json().await?;
    let raw_txs = resp["result"].as_array().cloned().unwrap_or_default();
    let mut txs = vec![];

    for tx in &raw_txs {
        if tx["hash"].as_str() == since_txid.as_deref() {
            break;
        }
        if let Some(transaction) = map_ethereum_tx(tx, address) {
            txs.push(transaction);
        }
    }

    Ok(txs)
}

pub fn map_ethereum_tx(result: &serde_json::Value, my_address: &str) -> Option<Transaction> {
    let address_from = result["from"].as_str()?.to_string();
    let address_to = result["to"].as_str()?.to_string();

    let my = my_address.to_lowercase();
    let direction = if address_from.to_lowercase() == my {
        Direction::Sent
    } else {
        Direction::Received
    };

    let value_wei: u128 = result["value"].as_str()?.parse().ok()?;
    let amount = value_wei as f64 / WEI_PER_ETH as f64;

    let gas_used: u128 = result["gasUsed"].as_str()?.parse().ok()?;
    let gas_price: u128 = result["gasPrice"].as_str()?.parse().ok()?;
    let fee = (gas_used * gas_price) as f64 / WEI_PER_ETH as f64;

    let status = if result["isError"].as_str().unwrap_or("0") == "0" {
        Status::Success
    } else {
        Status::Rejected
    };

    Some(Transaction {
        txid: result["hash"].as_str()?.to_string(),
        network: Network::Eth,
        direction,
        status,
        timestamp: result["timeStamp"].as_str()?.to_string(),
        amount,
        address_from,
        address_to,
        fee,
    })
}