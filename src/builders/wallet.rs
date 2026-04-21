use bip39::{Mnemonic, Language};
use anyhow::{Ok, Result};
use bitcoin::consensus::serialize;
use rand::RngCore;

#[derive(Debug)]
pub struct Wallet {
    pub mnemonic: String,
}

impl Wallet {
    pub fn generate() -> Result<Self> {
        // 16 random bytes = 128 bits of entropy = 12 words
        let mut entropy = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);

        // Convert random bytes → 12 human-readable BIP39 words
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)?;

        Ok(Wallet {
            mnemonic: mnemonic.to_string(),
        })
    }

    pub fn from_mnemonic(phrase: &str) -> Result<Self> {
        let wallet = Wallet{mnemonic: phrase.to_string()};
        Ok(wallet)
    }

    // Convert the mnemonic → raw 64 bytes
    pub fn seed_bytes(&self) -> Result<Vec<u8>> {
        // parse the mnemonic and call .to_seed("")
        let mnemonic = Mnemonic::parse_in(Language::English, &self.mnemonic)?;
        let seed = mnemonic.to_seed("");
        Ok(seed.to_vec())
    }
}
