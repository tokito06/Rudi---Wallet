use bitcoin::{
    Address, Network, PrivateKey, PublicKey, bip32::{DerivationPath, Xpriv}, secp256k1::Secp256k1
};
use anyhow::{Ok, Result};
use std::str::FromStr;

pub fn derive_address(seed: &[u8]) -> Result<String> {

    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Testnet, seed)?;
    let path = DerivationPath::from_str("m/44'/0'/0'/0/0")?;
    let child = master.derive_priv(&secp, &path)?;
    let pubkey = PublicKey::from_private_key(&secp, &child.to_priv());
    let address = Address::p2pkh(&pubkey, Network::Testnet);
    Ok(address.to_string())

}



pub fn derive_private_key(seed: &[u8]) -> Result<PrivateKey> {
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Testnet, seed)?;
    let path = DerivationPath::from_str("m/44'/0'/0'/0/0")?;
    let child = master.derive_priv(&secp, &path)?;
    let priv_key = child.to_priv();
    Ok(priv_key)
}