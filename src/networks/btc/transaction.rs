use bitcoin::{
    Transaction, TxIn, TxOut,        // transaction building blocks
    OutPoint, ScriptBuf,             // input/output details
    Sequence, Witness,               // required fields
    Amount,                          // amount type
    PrivateKey, PublicKey,           // for signing
    secp256k1::Secp256k1,           // for signing
    Address,                         // to parse recipient address
    Network,                         // testnet
    sighash::SighashCache,          // for creating the signature hash
    EcdsaSighashType,               // signature type
};

use std::str::FromStr;
use anyhow::Result;
use bitcoin::consensus::encode::serialize_hex;

use bitcoin::hashes::Hash;
use crate::networks::btc::bitcoin_network::Utxo;


pub fn send_btc( private_key: PrivateKey, utxos: Vec<Utxo>, recipient: &str, amount_sat: u64, change_address: &str,) -> Result<String> {
    let inputs: Vec<TxIn> = utxos.iter().map(|utxo| {
    TxIn {
        previous_output: OutPoint {
            txid: utxo.txid.parse().unwrap(),
            vout: utxo.vout,
        },
        script_sig: ScriptBuf::new(),  // empty for now, filled when signing
        sequence: Sequence::MAX,
        witness: Witness::default(),
    }
    }).collect();


    let total_input = utxos.iter().map(|u| u.value).sum();

    let fee = 1000;

    let change = {
        if amount_sat + fee < total_input {
            total_input - amount_sat - fee
        }
        else {
            anyhow::bail!("Not enough tokens");
        }
    };

    let recipient_address = Address::from_str(recipient)?.require_network(Network::Testnet)?;
    let change_address = Address::from_str(change_address)?.require_network(Network::Testnet)?;

    let outputs = vec![
        TxOut {
            value: Amount::from_sat(amount_sat),
            script_pubkey: recipient_address.script_pubkey(),
        },
        TxOut {
            value: Amount::from_sat(change),
            script_pubkey: change_address.script_pubkey(),
        },
    ];

    let mut tx = Transaction {
    version: bitcoin::transaction::Version::TWO,
    lock_time: bitcoin::absolute::LockTime::ZERO,
    input: inputs,
    output: outputs,
    };


    let secp = Secp256k1::new();
    let pubkey = PublicKey::from_private_key(&secp, &private_key);
    let script = Address::p2pkh(&pubkey, Network::Testnet).script_pubkey();

    for i in 0..tx.input.len() {
        let mut cache = SighashCache::new(&tx);
        let sighash = cache.legacy_signature_hash(
            i,
            &script,
            EcdsaSighashType::All.to_u32(),
        )?;

        let msg = bitcoin::secp256k1::Message::from_digest(*sighash.as_byte_array());
        let sig = secp.sign_ecdsa(&msg, &private_key.inner);

        let mut sig_bytes = sig.serialize_der().to_vec();
        sig_bytes.push(EcdsaSighashType::All.to_u32() as u8);

        tx.input[i].script_sig = bitcoin::ScriptBuf::builder()
            .push_slice(bitcoin::script::PushBytesBuf::try_from(sig_bytes).unwrap().as_push_bytes())
            .push_key(&pubkey)
            .into_script();
    }

    let tx_hex = serialize_hex(&tx);
    let txid = crate::networks::btc::bitcoin_network::broadcast_tx(&tx_hex)?;
    Ok(txid)
}