use alloy::{
    providers::{Provider, ProviderBuilder},
    primitives::{Address, U256},
    network::{EthereumWallet, TransactionBuilder},  // ← add TransactionBuilder
    rpc::types::TransactionRequest,
};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;

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

