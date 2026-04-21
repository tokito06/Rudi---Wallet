use alloy::{
    signers::Signer,
    signers::local::{PrivateKeySigner},
    
};

use bitcoin::{bip32::{DerivationPath, Xpriv}, Network, secp256k1::Secp256k1};

use bip39::{Mnemonic, Language};

use anyhow::{Ok, Result};
use std::str::FromStr;

pub fn derive_address(seed: &[u8]) -> Result<String> {
    let signer = derive_private_key(seed)?;
    Ok(signer.address().to_string())
}

pub fn derive_private_key(seed: &[u8]) -> Result<PrivateKeySigner> {
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Testnet, seed)?;
    let path = DerivationPath::from_str("m/44'/60'/0'/0/0")?;
    let child = master.derive_priv(&secp, &path)?;
    let raw_bytes = child.to_priv().to_bytes();
    let bytes_array: [u8; 32] = raw_bytes.as_slice().try_into()?;
    let signer = PrivateKeySigner::from_bytes(&bytes_array.into())?;;
    Ok(signer)
}