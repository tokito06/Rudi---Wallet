use ed25519_dalek::{SigningKey, VerifyingKey};
use anyhow::Result;
use bs58;

pub struct SolanaKeypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

fn derive_keypair(seed: &[u8]) -> Result<SolanaKeypair> {
    if seed.len() < 32 {
        anyhow::bail!("Seed must be at least 32 bytes");
    }
    let secret_bytes: [u8; 32] = seed[..32].try_into()?;
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = signing_key.verifying_key();
    Ok(SolanaKeypair { signing_key, verifying_key })
}

pub fn derive_address(seed: &[u8]) -> Result<String> {
    let keypair = derive_keypair(seed)?;
    Ok(bs58::encode(keypair.verifying_key.as_bytes()).into_string())
}

pub fn derive_private_key(seed: &[u8]) -> Result<SolanaKeypair> {
    derive_keypair(seed)
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
    fn test_derive_address_is_base58() {
        let address = derive_address(&test_seed()).unwrap();
        assert!(bs58::decode(&address).into_vec().is_ok());
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
        assert!(derive_address(&vec![0u8; 16]).is_err());
    }

    #[test]
    fn test_derive_address_empty_seed() {
        assert!(derive_address(&[]).is_err());
    }

    #[test]
    fn test_derive_address_exact_32_bytes() {
        assert!(derive_address(&vec![0u8; 32]).is_ok());
    }

    #[test]
    fn test_derive_private_key_returns_ok() {
        assert!(derive_private_key(&test_seed()).is_ok());
    }

    #[test]
    fn test_derive_private_key_is_deterministic() {
        let key1 = derive_private_key(&test_seed()).unwrap();
        let key2 = derive_private_key(&test_seed()).unwrap();
        assert_eq!(key1.verifying_key.as_bytes(), key2.verifying_key.as_bytes());
    }

    #[test]
    fn test_derive_private_key_different_seeds_give_different_keys() {
        let key1 = derive_private_key(&vec![0u8; 64]).unwrap();
        let key2 = derive_private_key(&vec![1u8; 64]).unwrap();
        assert_ne!(key1.verifying_key.as_bytes(), key2.verifying_key.as_bytes());
    }

    #[test]
    fn test_derive_private_key_seed_too_short() {
        assert!(derive_private_key(&vec![0u8; 16]).is_err());
    }

    #[test]
    fn test_address_matches_private_key() {
        let seed = test_seed();
        let keypair = derive_private_key(&seed).unwrap();
        let expected = bs58::encode(keypair.verifying_key.as_bytes()).into_string();
        let address = derive_address(&seed).unwrap();
        assert_eq!(address, expected);
    }

    #[test]
    fn test_derive_address_length() {
        let address = derive_address(&test_seed()).unwrap();
        assert!(address.len() >= 32 && address.len() <= 44, "Unexpected address length: {}", address.len());
    }

    #[test]
    fn test_signing_key_bytes_match_seed() {
        let seed = test_seed();
        let keypair = derive_private_key(&seed).unwrap();
        let expected_bytes: [u8; 32] = seed[..32].try_into().unwrap();
        assert_eq!(keypair.signing_key.to_bytes(), expected_bytes);
    }
}