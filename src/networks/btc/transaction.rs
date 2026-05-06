use bitcoin::{
    Transaction, TxIn, TxOut,
    OutPoint, ScriptBuf,
    Sequence, Witness,
    Amount,
    PrivateKey, PublicKey,
    secp256k1::Secp256k1,
    Address,
    Network,
    sighash::SighashCache,
    EcdsaSighashType,
};
use std::str::FromStr;
use anyhow::Result;
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::Hash;
use crate::networks::btc::bitcoin_network::Utxo;

const DUST_LIMIT: u64 = 546;
const SAT_PER_VBYTE: u64 = 10;

fn estimate_fee(input_count: usize, output_count: usize) -> u64 {
    let tx_size = (input_count * 148 + output_count * 34 + 10) as u64;
    tx_size * SAT_PER_VBYTE
}

pub async fn send_btc(
    private_key: PrivateKey,
    utxos: Vec<Utxo>,
    recipient: &str,
    amount_sat: u64,
    change_address: &str,
) -> Result<String> {
    let inputs: Vec<TxIn> = utxos.iter().map(|utxo| -> Result<TxIn> {
        Ok(TxIn {
            previous_output: OutPoint {
                txid: utxo.txid.parse()?,
                vout: utxo.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        })
    }).collect::<Result<Vec<_>>>()?;

    let total_input: u64 = utxos.iter().map(|u| u.value).sum();
    let fee = estimate_fee(inputs.len(), 2);

    if amount_sat + fee > total_input {
        anyhow::bail!("Not enough funds. Required: {}, Available: {}", amount_sat + fee, total_input);
    }

    let change = total_input - amount_sat - fee;

    if change > 0 && change < DUST_LIMIT {
        anyhow::bail!("Change amount {} is below dust limit {}", change, DUST_LIMIT);
    }

    let recipient_address = Address::from_str(recipient)?.require_network(Network::Testnet)?;
    let change_address = Address::from_str(change_address)?.require_network(Network::Testnet)?;

    let mut outputs = vec![
        TxOut {
            value: Amount::from_sat(amount_sat),
            script_pubkey: recipient_address.script_pubkey(),
        },
    ];

    if change >= DUST_LIMIT {
        outputs.push(TxOut {
            value: Amount::from_sat(change),
            script_pubkey: change_address.script_pubkey(),
        });
    }

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
        let cache = SighashCache::new(&tx);
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
            .push_slice(
                bitcoin::script::PushBytesBuf::try_from(sig_bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to push sig bytes: {}", e))?
                    .as_push_bytes()
            )
            .push_key(&pubkey)
            .into_script();
    }

    let tx_hex = serialize_hex(&tx);
    let txid = crate::networks::btc::bitcoin_network::broadcast_tx(&tx_hex).await?;
    Ok(txid)
}