use bip39::{Mnemonic, Language};
use anyhow::Result;
use rand::RngCore;

#[derive(Debug)]
pub struct Wallet {
    pub mnemonic: String,
}

impl Wallet {
    pub fn generate() -> Result<Self> {
        let mut entropy = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)?;
        Ok(Wallet { mnemonic: mnemonic.to_string() })
    }

    pub fn from_mnemonic(phrase: &str) -> Result<Self> {
        Mnemonic::parse_in(Language::English, phrase)?;
        Ok(Wallet { mnemonic: phrase.to_string() })
    }

    pub fn seed_bytes(&self, passphrase: &str) -> Result<Vec<u8>> {
        let mnemonic = Mnemonic::parse_in(Language::English, &self.mnemonic)?;
        Ok(mnemonic.to_seed(passphrase).to_vec())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn valid_mnemonic() -> &'static str {
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    }

    #[test]
    fn test_generate_returns_ok() {
        assert!(Wallet::generate().is_ok());
    }

    #[test]
    fn test_generate_mnemonic_has_12_words() {
        let wallet = Wallet::generate().unwrap();
        assert_eq!(wallet.mnemonic.split_whitespace().count(), 12);
    }

    #[test]
    fn test_generate_mnemonics_are_unique() {
        let wallet1 = Wallet::generate().unwrap();
        let wallet2 = Wallet::generate().unwrap();
        assert_ne!(wallet1.mnemonic, wallet2.mnemonic);
    }

    #[test]
    fn test_from_mnemonic_valid() {
        assert!(Wallet::from_mnemonic(valid_mnemonic()).is_ok());
    }

    #[test]
    fn test_from_mnemonic_stores_phrase() {
        let wallet = Wallet::from_mnemonic(valid_mnemonic()).unwrap();
        assert_eq!(wallet.mnemonic, valid_mnemonic());
    }

    #[test]
    fn test_from_mnemonic_invalid_phrase() {
        assert!(Wallet::from_mnemonic("not a valid mnemonic phrase at all").is_err());
    }

    #[test]
    fn test_from_mnemonic_empty_string() {
        assert!(Wallet::from_mnemonic("").is_err());
    }

    #[test]
    fn test_from_mnemonic_wrong_word() {
        assert!(Wallet::from_mnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon INVALID").is_err());
    }

    #[test]
    fn test_seed_bytes_returns_64_bytes() {
        let wallet = Wallet::from_mnemonic(valid_mnemonic()).unwrap();
        let seed = wallet.seed_bytes("").unwrap();
        assert_eq!(seed.len(), 64);
    }

    #[test]
    fn test_seed_bytes_is_deterministic() {
        let wallet = Wallet::from_mnemonic(valid_mnemonic()).unwrap();
        let seed1 = wallet.seed_bytes("").unwrap();
        let seed2 = wallet.seed_bytes("").unwrap();
        assert_eq!(seed1, seed2);
    }

    #[test]
    fn test_seed_bytes_different_passphrase_gives_different_seed() {
        let wallet = Wallet::from_mnemonic(valid_mnemonic()).unwrap();
        let seed1 = wallet.seed_bytes("").unwrap();
        let seed2 = wallet.seed_bytes("passphrase").unwrap();
        assert_ne!(seed1, seed2);
    }

    #[test]
    fn test_seed_bytes_different_mnemonics_give_different_seeds() {
        let wallet1 = Wallet::generate().unwrap();
        let wallet2 = Wallet::generate().unwrap();
        let seed1 = wallet1.seed_bytes("").unwrap();
        let seed2 = wallet2.seed_bytes("").unwrap();
        assert_ne!(seed1, seed2);
    }

    #[test]
    fn test_seed_bytes_known_value() {
        let wallet = Wallet::from_mnemonic(valid_mnemonic()).unwrap();
        let seed = wallet.seed_bytes("").unwrap();
        let expected = "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc19a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4";
        assert_eq!(hex::encode(&seed[..32]), &expected[..64]);
    }
}