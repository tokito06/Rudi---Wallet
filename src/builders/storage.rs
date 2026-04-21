use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

// What gets saved to disk (all sensitive data is inside `encrypted`)
#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub version: u8,
    pub salt: String,       // random, used in PBKDF2
    pub nonce: String,      // random, used in AES-GCM
    pub encrypted: String,  // the actual secret data, encrypted
}

// The secret data — only lives in RAM, never written to disk as plaintext
#[derive(Serialize, Deserialize, Debug)]
pub struct WalletData {
    pub mnemonic: String,
}

fn wallet_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".crypto-wallet.dat")
}

pub fn wallet_exists() -> bool {
    wallet_path().exists()
}

pub fn save_wallet(mnemonic: &str, password: &str) -> Result<()> {
    // Step 1: prepare the data we want to encrypt
    let data = WalletData { mnemonic: mnemonic.to_string() };
    let plaintext = serde_json::to_vec(&data)?;

    // Step 2: generate a random salt (makes every encryption unique)
    let salt: [u8; 16] = {
        let mut s = [0u8; 16];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut s);
        s
    };

    // Step 3: derive a 256-bit encryption key from the password
    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, 100_000, &mut key_bytes);

    // Step 4: encrypt with AES-256-GCM
    let key    = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Step 5: save salt + nonce + ciphertext to disk as JSON
    let file = WalletFile {
        version: 1,
        salt: hex::encode(salt),
        nonce: hex::encode(nonce),
        encrypted: hex::encode(ciphertext),
    };

    let path = wallet_path();
    fs::write(&path, serde_json::to_string_pretty(&file)?)
        .with_context(|| format!("Could not write to {:?}", path))?;

    println!("Wallet saved to: {:?}", path);
    Ok(())
}

pub fn load_wallet(password: &str) -> Result<WalletData> {
    let path = wallet_path();
    if !path.exists() {
        anyhow::bail!("No wallet found. Create one first.");
    }

    // Read and parse the JSON file
    let json = fs::read_to_string(&path)?;
    let file: WalletFile = serde_json::from_str(&json)
        .context("Wallet file is corrupted")?;

    // Decode hex values back to bytes
    let salt       = hex::decode(&file.salt)?;
    let nonce_raw  = hex::decode(&file.nonce)?;
    let ciphertext = hex::decode(&file.encrypted)?;

    // Re-derive the same key using the stored salt + the password
    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, 100_000, &mut key_bytes);

    // Decrypt — fails with clear error if password is wrong
    let key    = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Nonce::from_slice(&nonce_raw);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Wrong password or corrupted file."))?;

    let data: WalletData = serde_json::from_slice(&plaintext)?;
    Ok(data)
}