use crate::builders::storage;
use regex::Regex;

pub fn change_password(old_pass: &str, new_pass: &str) -> Result<()> {
    let path = wallet_path();
    if !path.exists() {
        anyhow::bail!("No wallet found.");
    }

    let json = fs::read_to_string(&path)?;
    let file: WalletFile = serde_json::from_str(&json)
        .context("Wallet file is corrupted")?;

    let mut data = decrypt_wallet_data(&file, old_pass)?;

    let result = save_wallet(&data.mnemonic, new_pass);

    data.mnemonic.zeroize();

    result
}

fn strong_password(pass: &str) -> bool {
    pass.len() >= 8
        && Regex::new(r"[A-Z]").unwrap().is_match(pass)
        && Regex::new(r"[a-z]").unwrap().is_match(pass)
        && Regex::new(r"[0-9]").unwrap().is_match(pass)
        && Regex::new(r"[!?.+\-*/@$^]").unwrap().is_match(pass)
}