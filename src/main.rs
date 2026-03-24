use crate::wallet::Wallet;


mod transaction;
mod bitcoin;
mod wallet;
mod storage;
mod network;
fn main() {

    let password = "cooling cool";
    
    if !storage::wallet_exists() {
        let wallet = Wallet::generate().expect("error while creating a seed phrase");
        let mnemonic = wallet.mnemonic;
        storage::save_wallet(&mnemonic, password).expect("fail on save the wallet");
        storage::load_wallet(password).expect("failed to load the wallet");
    }

    let loaded = storage::load_wallet(password).expect("failed to load the wallet");
    let wallet = wallet::Wallet::from_mnemonic(&loaded.mnemonic).expect("Failed to parse mnemonic");
    let seed = wallet.seed_bytes().unwrap();
    let address = bitcoin::derive_address(&seed).expect("Failed to derive address");
    println!("Address: {}", address);

    let private_key = bitcoin::derive_private_key(&seed).expect("fail");

    let balance = network::get_btc_balance(&address);
    println!("{:?}", balance.unwrap()); 


    let utxos = network::get_utxos(&address);
    let amount_of_tokens = 10000000000000000;
    let recipient = "mg5T2hKxW2en7GbezCxco1ohdd9PzFuAJC";

    let tx_hex = transaction::send_btc(private_key, utxos.unwrap(), recipient, amount_of_tokens, &address);

    println!("transaction was made");

    let balance = network::get_btc_balance(&address);
    
    println!("{:?}", balance.unwrap());
}