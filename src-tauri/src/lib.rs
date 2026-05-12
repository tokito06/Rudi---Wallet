use tauri::State;
use std::sync::Mutex;
use bitcoin::Network;
use rust_project_rudi::session::Session;

pub struct AppState {
    pub session: Mutex<Option<Session>>,
    pub password: Mutex<Option<String>>,
}

// Extracts the seed from an active, non-expired session, refreshing its timeout.
fn get_seed(session_lock: &Mutex<Option<Session>>) -> Result<Vec<u8>, String> {
    let mut guard = session_lock.lock().unwrap();
    let expired = guard.as_ref().map_or(true, |s| s.is_expired());
    if expired {
        *guard = None;
        return Err("Session expired. Please unlock your wallet.".to_string());
    }
    let seed = guard.as_ref().unwrap().seed.clone();
    guard.as_mut().unwrap().refresh();
    Ok(seed)
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn wallet_exists() -> bool {
    rust_project_rudi::builders::storage::wallet_exists()
}

#[tauri::command]
fn create_wallet(
    state: State<AppState>,
    password: String,
) -> Result<Vec<String>, String> {
    let wallet = rust_project_rudi::builders::wallet::Wallet::generate()
        .map_err(|e| e.to_string())?;

    rust_project_rudi::builders::storage::save_wallet(&wallet.mnemonic, &password)
        .map_err(|e| e.to_string())?;

    let seed = wallet.seed_bytes("").map_err(|e| e.to_string())?;
    *state.session.lock().unwrap() = Some(Session::new(seed));
    *state.password.lock().unwrap() = Some(password);

    let words: Vec<String> = wallet.mnemonic
        .split_whitespace()
        .map(|w| w.to_string())
        .collect();

    Ok(words)
}

#[tauri::command]
fn import_wallet(
    state: State<AppState>,
    mnemonic: String,
    password: String,
) -> Result<bool, String> {
    let mnemonic_trimmed = mnemonic.trim();
    let word_count = mnemonic_trimmed.split_whitespace().count();
    if word_count != 12 && word_count != 24 {
        return Err("Invalid seed phrase. Must be 12 or 24 words.".to_string());
    }

    let wallet = rust_project_rudi::builders::wallet::Wallet::from_mnemonic(mnemonic_trimmed)
        .map_err(|e| e.to_string())?;

    rust_project_rudi::builders::storage::save_wallet(mnemonic_trimmed, &password)
        .map_err(|e| e.to_string())?;

    let seed = wallet.seed_bytes("").map_err(|e| e.to_string())?;
    *state.session.lock().unwrap() = Some(Session::new(seed));
    *state.password.lock().unwrap() = Some(password);

    Ok(true)
}

#[tauri::command]
fn unlock_wallet(
    state: State<AppState>,
    password: String,
) -> Result<bool, String> {
    let loaded = rust_project_rudi::builders::storage::load_wallet(&password)
        .map_err(|_| "Wrong password".to_string())?;

    let wallet = rust_project_rudi::builders::wallet::Wallet::from_mnemonic(&loaded.mnemonic)
        .map_err(|e| e.to_string())?;

    let seed = wallet.seed_bytes("").map_err(|e| e.to_string())?;
    *state.session.lock().unwrap() = Some(Session::new(seed));
    *state.password.lock().unwrap() = Some(password);

    Ok(true)
}

#[tauri::command]
fn check_session(state: State<AppState>) -> bool {
    let mut guard = state.session.lock().unwrap();
    let expired = guard.as_ref().map_or(true, |s| s.is_expired());
    if expired {
        *guard = None;
        return false;
    }
    guard.as_mut().unwrap().refresh();
    true
}

#[tauri::command]
fn lock_wallet(state: State<AppState>) {
    *state.session.lock().unwrap() = None;
    *state.password.lock().unwrap() = None;
}

#[tauri::command]
fn get_address(
    state: State<AppState>,
    network: String,
) -> Result<String, String> {
    let seed = get_seed(&state.session)?;

    match network.as_str() {
        "btc" => rust_project_rudi::tokens::bitcoin::derive_address(&seed, Network::Testnet, "m/44'/0'/0'/0/0")
            .map_err(|e| e.to_string()),
        "sol" => rust_project_rudi::tokens::solana::derive_address(&seed)
            .map_err(|e| e.to_string()),
        "eth" => rust_project_rudi::tokens::ethereum::derive_address(&seed)
            .map_err(|e| e.to_string()),
        _ => Err("Unknown network".to_string()),
    }
}

#[tauri::command]
async fn get_balance(
    state: State<'_, AppState>,
    network: String,
) -> Result<f64, String> {
    let seed = get_seed(&state.session)?;

    match network.as_str() {
        "btc" => {
            let address = rust_project_rudi::tokens::bitcoin::derive_address(&seed, Network::Testnet, "m/44'/0'/0'/0/0")
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::btc::bitcoin_network::get_btc_balance(&address).await
                .map_err(|e| e.to_string())
        }
        "sol" => {
            let address = rust_project_rudi::tokens::solana::derive_address(&seed)
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::solana_network::get_sol_balance(&address).await
                .map_err(|e| e.to_string())
        }
        _ => Err("Unknown network".to_string()),
    }
}

#[tauri::command]
async fn get_eth_balance(
    state: State<'_, AppState>,
) -> Result<f64, String> {
    let seed = get_seed(&state.session)?;

    let address = rust_project_rudi::tokens::ethereum::derive_address(&seed)
        .map_err(|e| e.to_string())?;

    rust_project_rudi::networks::ethereum_network::get_eth_balance(&address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_transaction(
    state: State<'_, AppState>,
    network: String,
    recipient: String,
    amount: f64,
) -> Result<String, String> {
    let seed = get_seed(&state.session)?;

    match network.as_str() {
        "btc" => {
            let private_key = rust_project_rudi::tokens::bitcoin::derive_private_key(&seed, Network::Testnet, "m/44'/0'/0'/0/0")
                .map_err(|e| e.to_string())?;
            let sender_address = rust_project_rudi::tokens::bitcoin::derive_address(&seed, Network::Testnet, "m/44'/0'/0'/0/0")
                .map_err(|e| e.to_string())?;

            rust_project_rudi::helpers::making_tx::Network::Btc.send(
                rust_project_rudi::helpers::making_tx::Key::Btc(private_key),
                &sender_address,
                &recipient,
                amount,
                &sender_address,
            ).await.map_err(|e| e.to_string())
        }
        "sol" => {
            let keypair = rust_project_rudi::tokens::solana::derive_private_key(&seed)
                .map_err(|e| e.to_string())?;
            let sender_address = rust_project_rudi::tokens::solana::derive_address(&seed)
                .map_err(|e| e.to_string())?;

            rust_project_rudi::helpers::making_tx::Network::Sol.send(
                rust_project_rudi::helpers::making_tx::Key::Sol(keypair.signing_key),
                &sender_address,
                &recipient,
                amount,
                &sender_address,
            ).await.map_err(|e| e.to_string())
        }
        "eth" => {
            let signer = rust_project_rudi::tokens::ethereum::derive_private_key(&seed)
                .map_err(|e| e.to_string())?;
            let sender_address = rust_project_rudi::tokens::ethereum::derive_address(&seed)
                .map_err(|e| e.to_string())?;

            rust_project_rudi::helpers::making_tx::Network::Eth.send(
                rust_project_rudi::helpers::making_tx::Key::Eth(signer),
                &sender_address,
                &recipient,
                amount,
                &sender_address,
            ).await.map_err(|e| e.to_string())
        }
        _ => Err("Unknown network".to_string()),
    }
}

#[tauri::command]
async fn broadcast_tx(tx_hex: String) -> Result<String, String> {
    rust_project_rudi::networks::btc::bitcoin_network::broadcast_tx(&tx_hex)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_receive_address(
    state: State<AppState>,
    network: String,
) -> Result<String, String> {
    let seed = get_seed(&state.session)?;

    match network.as_str() {
        "btc" => rust_project_rudi::tokens::bitcoin::derive_address(&seed, Network::Testnet, "m/44'/0'/0'/0/0")
            .map_err(|e| e.to_string()),
        "sol" => rust_project_rudi::tokens::solana::derive_address(&seed)
            .map_err(|e| e.to_string()),
        "eth" => rust_project_rudi::tokens::ethereum::derive_address(&seed)
            .map_err(|e| e.to_string()),
        _ => Err("Unknown network".to_string()),
    }
}

#[tauri::command]
async fn get_transaction_history(
    state: State<'_, AppState>,
    network: String,
    since_txid: Option<String>,
) -> Result<Vec<rust_project_rudi::helpers::types::Transaction>, String> {
    let seed = get_seed(&state.session)?;

    match network.as_str() {
        "btc" => {
            let address = rust_project_rudi::tokens::bitcoin::derive_address(&seed, Network::Testnet, "m/44'/0'/0'/0/0")
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::btc::bitcoin_network::fetch_btc_history(&address, since_txid)
                .await
                .map_err(|e| e.to_string())
        }
        "sol" => {
            let address = rust_project_rudi::tokens::solana::derive_address(&seed)
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::solana_network::fetch_sol_history(&address, since_txid)
                .await
                .map_err(|e| e.to_string())
        }
        "eth" => {
            let address = rust_project_rudi::tokens::ethereum::derive_address(&seed)
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::ethereum_network::fetch_eth_history(&address, since_txid)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err("Unknown network".to_string()),
    }
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            session: Mutex::new(None),
            password: Mutex::new(None),
        })
        .setup(|app| {
            let _ = app;
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            wallet_exists,
            create_wallet,
            unlock_wallet,
            import_wallet,
            check_session,
            lock_wallet,
            get_address,
            get_balance,
            get_eth_balance,
            send_transaction,
            get_receive_address,
            get_transaction_history,
            broadcast_tx,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
