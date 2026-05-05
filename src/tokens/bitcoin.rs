use bitcoin::{
    Address, Network, PrivateKey, PublicKey,
    bip32::{DerivationPath, Xpriv},
    secp256k1::Secp256k1,
};
use anyhow::Result;
use std::str::FromStr;

fn derive_child(seed: &[u8], network: Network, path: &str) -> Result<Xpriv> {
    if seed.len() < 16 || seed.len() > 64 {
        anyhow::bail!("Seed must be between 16 and 64 bytes");
    }
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(network, seed)?;
    let path = DerivationPath::from_str(path)?;
    Ok(master.derive_priv(&secp, &path)?)
}

pub fn derive_address(seed: &[u8], network: Network, path: &str) -> Result<String> {
    let secp = Secp256k1::new();
    let child = derive_child(seed, network, path)?;
    let pubkey = PublicKey::from_private_key(&secp, &child.to_priv());
    let address = Address::p2pkh(&pubkey, network);
    Ok(address.to_string())
}

pub fn derive_private_key(seed: &[u8], network: Network, path: &str) -> Result<PrivateKey> {
    let child = derive_child(seed, network, path)?;
    Ok(child.to_priv())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::Secp256k1;

    const DEFAULT_PATH: &str = "m/44'/0'/0'/0/0";

    fn test_seed() -> Vec<u8> {
        vec![0u8; 64]
    }

    #[test]
    fn test_derive_address_returns_ok() {
        let result = derive_address(&test_seed(), Network::Testnet, DEFAULT_PATH);
        assert!(result.is_ok());
    }

    #[test]
    fn test_derive_address_starts_with_testnet_prefix() {
        let address = derive_address(&test_seed(), Network::Testnet, DEFAULT_PATH).unwrap();
        assert!(
            address.starts_with('m') || address.starts_with('n'),
            "Expected testnet address, got: {}",
            address
        );
    }

    #[test]
    fn test_derive_address_is_deterministic() {
        let addr1 = derive_address(&test_seed(), Network::Testnet, DEFAULT_PATH).unwrap();
        let addr2 = derive_address(&test_seed(), Network::Testnet, DEFAULT_PATH).unwrap();
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_derive_address_different_seeds_give_different_addresses() {
        let addr1 = derive_address(&vec![0u8; 64], Network::Testnet, DEFAULT_PATH).unwrap();
        let addr2 = derive_address(&vec![1u8; 64], Network::Testnet, DEFAULT_PATH).unwrap();
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_derive_address_invalid_seed_too_short() {
        let result = derive_address(&[], Network::Testnet, DEFAULT_PATH);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_address_invalid_seed_too_long() {
        let result = derive_address(&vec![0u8; 65], Network::Testnet, DEFAULT_PATH);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_private_key_returns_ok() {
        let result = derive_private_key(&test_seed(), Network::Testnet, DEFAULT_PATH);
        assert!(result.is_ok());
    }

    #[test]
    fn test_derive_private_key_is_deterministic() {
        let key1 = derive_private_key(&test_seed(), Network::Testnet, DEFAULT_PATH).unwrap();
        let key2 = derive_private_key(&test_seed(), Network::Testnet, DEFAULT_PATH).unwrap();
        assert_eq!(key1.to_bytes(), key2.to_bytes());
    }

    #[test]
    fn test_derive_private_key_different_seeds_give_different_keys() {
        let key1 = derive_private_key(&vec![0u8; 64], Network::Testnet, DEFAULT_PATH).unwrap();
        let key2 = derive_private_key(&vec![1u8; 64], Network::Testnet, DEFAULT_PATH).unwrap();
        assert_ne!(key1.to_bytes(), key2.to_bytes());
    }

    #[test]
    fn test_derive_private_key_network_is_testnet() {
        let key = derive_private_key(&test_seed(), Network::Testnet, DEFAULT_PATH).unwrap();
        assert_eq!(key.network, Network::Testnet);
    }

    #[test]
    fn test_derive_private_key_invalid_seed_too_short() {
        let result = derive_private_key(&[], Network::Testnet, DEFAULT_PATH);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_private_key_invalid_seed_too_long() {
        let result = derive_private_key(&vec![0u8; 65], Network::Testnet, DEFAULT_PATH);
        assert!(result.is_err());
    }

    #[test]
    fn test_private_key_matches_derived_address() {
        let seed = test_seed();
        let secp = Secp256k1::new();

        let priv_key = derive_private_key(&seed, Network::Testnet, DEFAULT_PATH).unwrap();
        let pub_key = PublicKey::from_private_key(&secp, &priv_key);
        let expected_address = Address::p2pkh(&pub_key, Network::Testnet).to_string();
        let derived_address = derive_address(&seed, Network::Testnet, DEFAULT_PATH).unwrap();

        assert_eq!(derived_address, expected_address);
    }

    #[test]
    fn test_derive_address_mainnet_prefix() {
        let address = derive_address(&test_seed(), Network::Bitcoin, DEFAULT_PATH).unwrap();
        assert!(
            address.starts_with('1'),
            "Expected mainnet address, got: {}",
            address
        );
    }
}