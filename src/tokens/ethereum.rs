use alloy::{
    signers::Signer,
    signers::local::PrivateKeySigner,
};
use bitcoin::{
    bip32::{DerivationPath, Xpriv},
    Network,
    secp256k1::Secp256k1,
};
use anyhow::Result;
use std::str::FromStr;

const DEFAULT_PATH: &str = "m/44'/60'/0'/0/0";

fn derive_child(seed: &[u8], path: &str) -> Result<Xpriv> {
    if seed.len() < 16 || seed.len() > 64 {
        anyhow::bail!("Seed must be between 16 and 64 bytes");
    }
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Testnet, seed)?;
    let path = DerivationPath::from_str(path)?;
    Ok(master.derive_priv(&secp, &path)?)
}

pub fn derive_private_key(seed: &[u8]) -> Result<PrivateKeySigner> {
    let child = derive_child(seed, DEFAULT_PATH)?;
    let raw_bytes = child.to_priv().to_bytes();
    let bytes_array: [u8; 32] = raw_bytes.as_slice().try_into()?;
    let signer = PrivateKeySigner::from_bytes(&bytes_array.into())?;
    Ok(signer)
}

pub fn derive_address(seed: &[u8]) -> Result<String> {
    let signer = derive_private_key(seed)?;
    Ok(signer.address().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seed() -> Vec<u8> {
        vec![0u8; 64]
    }

    #[test]
    fn test_derive_address_returns_ok() {
        assert!(derive_address(&test_seed()).is_ok());
    }

    #[test]
    fn test_derive_address_starts_with_0x() {
        let address = derive_address(&test_seed()).unwrap();
        assert!(address.starts_with("0x"), "Expected 0x prefix, got: {}", address);
    }

    #[test]
    fn test_derive_address_is_deterministic() {
        let addr1 = derive_address(&test_seed()).unwrap();
        let addr2 = derive_address(&test_seed()).unwrap();
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_derive_address_different_seeds_give_different_addresses() {
        let addr1 = derive_address(&vec![0u8; 64]).unwrap();
        let addr2 = derive_address(&vec![1u8; 64]).unwrap();
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_derive_address_seed_too_short() {
        assert!(derive_address(&[]).is_err());
    }

    #[test]
    fn test_derive_address_seed_too_long() {
        assert!(derive_address(&vec![0u8; 65]).is_err());
    }

    #[test]
    fn test_derive_private_key_returns_ok() {
        assert!(derive_private_key(&test_seed()).is_ok());
    }

    #[test]
    fn test_derive_private_key_is_deterministic() {
        let key1 = derive_private_key(&test_seed()).unwrap();
        let key2 = derive_private_key(&test_seed()).unwrap();
        assert_eq!(key1.address(), key2.address());
    }

    #[test]
    fn test_derive_private_key_different_seeds_give_different_keys() {
        let key1 = derive_private_key(&vec![0u8; 64]).unwrap();
        let key2 = derive_private_key(&vec![1u8; 64]).unwrap();
        assert_ne!(key1.address(), key2.address());
    }

    #[test]
    fn test_derive_private_key_seed_too_short() {
        assert!(derive_private_key(&[]).is_err());
    }

    #[test]
    fn test_derive_private_key_seed_too_long() {
        assert!(derive_private_key(&vec![0u8; 65]).is_err());
    }

    #[test]
    fn test_address_matches_private_key() {
        let seed = test_seed();
        let signer = derive_private_key(&seed).unwrap();
        let address_from_key = signer.address().to_string();
        let address_from_fn = derive_address(&seed).unwrap();
        assert_eq!(address_from_key, address_from_fn);
    }

    #[test]
    fn test_derive_address_length() {
        let address = derive_address(&test_seed()).unwrap();
        assert_eq!(address.len(), 42);
    }
}