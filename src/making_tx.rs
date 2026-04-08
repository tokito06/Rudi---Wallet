use crate::networks;
use anyhow::Result;
use bitcoin::PrivateKey;
use ed25519_dalek::SigningKey;

#[derive(PartialEq)]
pub enum Network {
    Btc,
    Sol,
    // Eth,
}

pub enum Key {
    Btc(PrivateKey),
    Sol(SigningKey),  // ← changed from Keypair to SigningKey
}

impl Network {
    pub fn send(
        &self,
        key: Key,
        sender_address: &str,
        recipient: &str,
        amount: f64,
        change_address: &str,
    ) -> Result<String> {
        match (self, key) {
            (Network::Btc, Key::Btc(private_key)) => {
                let utxos = networks::btc::bitcoin_network::get_utxos(sender_address)?;
                let amount_sat = (amount * 100_000_000.0) as u64;
                networks::btc::transaction::send_btc(
                    private_key,
                    utxos,
                    recipient,
                    amount_sat,
                    change_address,
                )
            }
            (Network::Sol, Key::Sol(signing_key)) => {
                networks::solana_network::send_sol(&signing_key, recipient, amount)
            }
            _ => anyhow::bail!("Key type does not match network"),
        }
    }
}