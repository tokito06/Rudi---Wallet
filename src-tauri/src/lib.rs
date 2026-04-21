use tauri::State;
use std::sync::Mutex;

// Session state — holds decrypted seed in RAM
pub struct AppState {
    pub seed: Mutex<Option<Vec<u8>>>,
    pub password: Mutex<Option<String>>,
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
    // generate wallet
    let wallet = rust_project_rudi::builders::wallet::Wallet::generate()
        .map_err(|e| e.to_string())?;

    // save encrypted to disk
    rust_project_rudi::builders::storage::save_wallet(&wallet.mnemonic, &password)
        .map_err(|e| e.to_string())?;

    // store seed in session
    let seed = wallet.seed_bytes().map_err(|e| e.to_string())?;
    *state.seed.lock().unwrap() = Some(seed);
    *state.password.lock().unwrap() = Some(password);

    // return seed words to show to user
    let words: Vec<String> = wallet.mnemonic
        .split_whitespace()
        .map(|w| w.to_string())
        .collect();

    Ok(words)
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

    let seed = wallet.seed_bytes().map_err(|e| e.to_string())?;

    *state.seed.lock().unwrap() = Some(seed);
    *state.password.lock().unwrap() = Some(password);

    Ok(true)
}

#[tauri::command]
fn get_address(
    state: State<AppState>,
    network: String,
) -> Result<String, String> {
    let seed_guard = state.seed.lock().unwrap();
    let seed = seed_guard.as_ref().ok_or("Wallet is locked")?;

    match network.as_str() {
        "btc" => rust_project_rudi::tokens::bitcoin::derive_address(seed)
            .map_err(|e| e.to_string()),
        "sol" => rust_project_rudi::tokens::solana::derive_address(seed)
            .map_err(|e| e.to_string()),
        "eth" => rust_project_rudi::tokens::ethereum::derive_address(seed)
            .map_err(|e| e.to_string()),
        _ => Err("Unknown network".to_string()),
    }
}

#[tauri::command]
fn get_balance(
    state: State<AppState>,
    network: String,
) -> Result<f64, String> {
    let seed_guard = state.seed.lock().unwrap();
    let seed = seed_guard.as_ref().ok_or("Wallet is locked")?;

    match network.as_str() {
        "btc" => {
            let address = rust_project_rudi::tokens::bitcoin::derive_address(seed)
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::btc::bitcoin_network::get_btc_balance(&address)
                .map_err(|e| e.to_string())
        }
        "sol" => {
            let address = rust_project_rudi::tokens::solana::derive_address(seed)
                .map_err(|e| e.to_string())?;
            rust_project_rudi::networks::solana_network::get_sol_balance(&address)
                .map_err(|e| e.to_string())
        }
        "eth" => {
            let address = rust_project_rudi::tokens::ethereum::derive_address(seed)
                .map_err(|e| e.to_string())?;
            // eth is async — handle separately
            Err("Use async command for ETH".to_string())
        }
        _ => Err("Unknown network".to_string()),
    }
}

#[tauri::command]
fn lock_wallet(state: State<AppState>) {
    *state.seed.lock().unwrap() = None;
    *state.password.lock().unwrap() = None;
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            seed: Mutex::new(None),
            password: Mutex::new(None),
        })
        .setup(|app| {
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
            get_address,
            get_balance,
            lock_wallet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}