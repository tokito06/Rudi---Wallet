use reqwest::blocking::Client;
use anyhow::Result;
use solana_sdk::{
    pubkey::Pubkey,
    system_transaction,
    native_token::sol_to_lamports,
    signer::keypair::Keypair,
};
use solana_client::rpc_client::RpcClient;


pub fn get_sol_balance(address: &str) -> Result<f64> {
    let client = Client::new();
    let url = "https://api.devnet.solana.com";
    let address = address.trim();

    println!("Address bytes: {:?}", address.as_bytes());
    println!("Address len: {}", address.len());

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [address]
    });

    let resp = client.post(url).json(&body).send()?;
    let text = resp.text()?;
    println!("Raw response: {}", text);

    let json: serde_json::Value = serde_json::from_str(&text)?;
    let lamports = json["result"]["value"].as_u64()
        .ok_or(anyhow::anyhow!("Failed to parse balance"))?;

    Ok(lamports as f64 / 1_000_000_000.0)
}



pub fn send_sol(keypair: Keypair, recipient: &str, amount_sol: f64,) -> Result<String> {
    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
    let last_block = rpc.get_latest_blockhash()?;
    let recipient_pubkey: Pubkey = recipient.trim().parse()
    .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    let tx = system_transaction::transfer(
    &keypair,
    &recipient_pubkey,               
    sol_to_lamports(amount_sol),
    last_block,             
    );
    let signature = rpc.send_and_confirm_transaction(&tx)?;
    Ok(signature.to_string())

}
