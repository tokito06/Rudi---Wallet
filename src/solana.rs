use ed25519_dalek::{SigningKey, VerifyingKey};
use anyhow::Result;
use bs58;

pub struct SolanaKeypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

pub fn derive_address(seed: &[u8]) -> Result<String> {
    // take first 32 bytes of seed as private key
    let secret_bytes: [u8; 32] = seed[..32].try_into()?;
    
    // create signing key from bytes
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    
    // public key = Solana address
    let verifying_key = signing_key.verifying_key();
    
    // encode as Base58
    Ok(bs58::encode(verifying_key.as_bytes()).into_string())
}

pub fn derive_private_key(seed: &[u8]) -> Result<SolanaKeypair> {
    let secret_bytes: [u8; 32] = seed[..32].try_into()?;
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = signing_key.verifying_key();
    
    Ok(SolanaKeypair { signing_key, verifying_key })
}