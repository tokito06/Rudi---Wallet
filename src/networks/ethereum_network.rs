use alloy::{
    providers::{Provider, ProviderBuilder},
    primitives::{Address, U256},
    network::{EthereumWallet, TransactionBuilder},
    rpc::types::TransactionRequest,
};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use reqwest::blocking::Client;
use crate::types::{Transaction, Direction, Status};
use crate::making_tx::Network;


pub async fn get_provider() -> Result<impl Provider> {
    let provider = ProviderBuilder::new()
        .connect("https://ethereum-sepolia-rpc.publicnode.com").await?;
    Ok(provider)
}

pub async fn get_eth_balance(address: &str)-> Result<f64>{
    let provider = get_provider().await?;
    let address: Address = address.parse()?;
    let balance = provider.get_balance(address).await?;
    Ok(balance.to::<u128>() as f64 / 1_000_000_000_000_000_000.0)
}

pub async fn calculate_gas(from: &str, to: &str, value: U256) -> Result<u128> {
    let provider = get_provider().await?;
    let from_address: Address = from.parse()?;
    let to_address: Address = to.parse()?;
    let trans_request = TransactionRequest::default()
        .with_from(from_address)
        .with_to(to_address)
        .with_value(value);
    let gas = provider.estimate_gas(trans_request).await?;
    Ok(gas as u128)
}
    

pub async fn send_eth(signer: PrivateKeySigner, recipient: &str, amount_eth: f64) -> Result<String> {
    let to_address: Address = recipient.parse()?;
    let from_address = signer.address();

    let amount_wei = U256::from((amount_eth * 1_000_000_000_000_000_000.0) as u128);

    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect("https://ethereum-sepolia-rpc.publicnode.com").await?;

    let gas = calculate_gas(&from_address.to_string(), recipient, amount_wei).await?;

    let transaction = TransactionRequest::default()
        .with_to(to_address)
        .with_from(from_address)
        .with_value(amount_wei)
        .with_gas_limit(gas as u64);
 
    let transaction_hash = provider.send_transaction(transaction).await?;
    
    Ok(transaction_hash.tx_hash().to_string())
}

pub fn fetch_eth_history(address: &str, since_txid: Option<String>) -> Result<Vec<Transaction>> {
    let client = Client::new();
    let url = format!(
        "https://eth-sepolia.blockscout.com/api?module=account&action=txlist&address={}&sort=desc",
        address
    );
    let resp: serde_json::Value = client.get(&url).send()?.json()?;
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
    let amount = value_wei as f64 / 1_000_000_000_000_000_000.0;

    let gas_used: u128 = result["gasUsed"].as_str()?.parse().ok()?;
    let gas_price: u128 = result["gasPrice"].as_str()?.parse().ok()?;
    let fee = (gas_used * gas_price) as f64 / 1_000_000_000_000_000_000.0;

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
