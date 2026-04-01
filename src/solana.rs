use anyhow::Result;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::signature::SeedDerivable;

pub fn derive_address(seed: &[u8]) -> Result<String> {
    let secret_bytes: &[u8; 32] = seed[..32].try_into()?;
    let keypair = Keypair::from_seed(secret_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create keypair: {}", e))?;
    Ok(keypair.pubkey().to_string())
}

pub fn derive_private_key(seed: &[u8]) -> Result<Keypair> {
    let secret_bytes: &[u8; 32] = seed[..32].try_into()?;
    let keypair = Keypair::from_seed(secret_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create keypair: {}", e))?;
    Ok(keypair)
}