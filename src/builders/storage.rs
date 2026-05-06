use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};
use zeroize::Zeroize;
use crate::builders::password;



const PBKDF2_ITERATIONS: u32 = 600_000;
const WALLET_VERSION: u8 = 1;
#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub version: u8,
    pub salt: String,
    pub nonce: String,
    pub encrypted: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletData {
    pub mnemonic: String,
}

fn wallet_path() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
    
    let home = std::fs::canonicalize(&home).unwrap_or(home);
    home.join(".crypto-wallet.dat")
}

pub fn wallet_exists() -> bool {
    wallet_path().exists()
}

fn validate_wallet_password_strength(password: &str) -> Result<()> {
     if !password::strong_password(password) {
            anyhow::bail!(
                "Password must include uppercase, lowercase, numeric, and special characters"
            );
    }
    Ok(())
}

pub fn save_wallet(mnemonic: &str, password: &str) -> Result<()> {
    validate_wallet_password_strength(password)?;

    let path = wallet_path();

    #[cfg(windows)]
    if path.exists() {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_readonly(false);
        fs::set_permissions(&path, perms)?;
    }

    let data = WalletData { mnemonic: mnemonic.to_string() };
    let plaintext = serde_json::to_vec(&data)?;

    let salt: [u8; 16] = {
        let mut s = [0u8; 16];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut s);
        s
    };

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut key_bytes);

    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let file = WalletFile {
        version: WALLET_VERSION,
        salt: hex::encode(salt),
        nonce: hex::encode(nonce),
        encrypted: hex::encode(ciphertext),
    };

    fs::write(&path, serde_json::to_string_pretty(&file)?)
        .with_context(|| format!("Could not write to {:?}", path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    #[cfg(windows)]
    {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

pub fn load_wallet(password: &str) -> Result<WalletData> {
    let path = wallet_path();
    if !path.exists() {
        anyhow::bail!("No wallet found. Create one first.");
    }

    let json = fs::read_to_string(&path)?;
    let file: WalletFile = serde_json::from_str(&json)
        .context("Wallet file is corrupted")?;

    if file.version != WALLET_VERSION {
        anyhow::bail!("Unsupported wallet version: {}", file.version);
    }

    decrypt_wallet_data(&file, password)
}

fn decrypt_wallet_data(file: &WalletFile, password: &str) -> Result<WalletData> {
    let salt = hex::decode(&file.salt)?;
    let nonce_raw = hex::decode(&file.nonce)?;
    let ciphertext = hex::decode(&file.encrypted)?;

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut key_bytes);

    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_raw);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Wrong password or corrupted file."))?;

    Ok(serde_json::from_slice(&plaintext)?)
}

pub fn change_password(old_pass: &str, new_pass: &str) -> Result<()> {
    if !password::strong_password(new_pass) {
        anyhow::bail!(
            "New password too weak (strength: {}). Avoid repeated/sequential characters and aim for 80+ bits of entropy.",
            password::password_strength(new_pass)
        );
    }

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


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serial_test::serial;

    fn setup(dir: &TempDir) {
        let path = dir.path().to_path_buf();
        std::fs::create_dir_all(&path).unwrap();
        std::env::set_var("HOME", path);
    }

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const TEST_PASSWORD: &str = "test_password_123";


    #[test]
    #[serial]
    fn test_wallet_exists_returns_false_when_no_file() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        assert!(!wallet_exists());
    }

    #[test]
    #[serial]
    fn test_wallet_exists_returns_true_after_save() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        assert!(wallet_exists());
    }

    #[test]
    #[serial]
    fn test_save_wallet_creates_file() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        assert!(wallet_path().exists());
    }

    #[test]
    #[serial]
    fn test_save_wallet_file_is_valid_json() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let json = std::fs::read_to_string(wallet_path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("version").is_some());
        assert!(parsed.get("salt").is_some());
        assert!(parsed.get("nonce").is_some());
        assert!(parsed.get("encrypted").is_some());
    }

    #[test]
    #[serial]
    fn test_save_wallet_version_is_1() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let json = std::fs::read_to_string(wallet_path()).unwrap();
        let file: WalletFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file.version, 1);
    }

    #[test]
    #[serial]
    fn test_load_wallet_correct_password() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let data = load_wallet(TEST_PASSWORD).unwrap();
        assert_eq!(data.mnemonic, TEST_MNEMONIC);
    }

    #[test]
    #[serial]
    fn test_load_wallet_wrong_password() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        assert!(load_wallet("wrong_password").is_err());
    }

    #[test]
    #[serial]
    fn test_load_wallet_no_file() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        assert!(load_wallet(TEST_PASSWORD).is_err());
    }

    #[test]
    #[serial]
    fn test_load_wallet_corrupted_file() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        std::fs::write(wallet_path(), "not valid json").unwrap();
        assert!(load_wallet(TEST_PASSWORD).is_err());
    }

    #[test]
    #[serial]
    fn test_load_wallet_unsupported_version() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let json = std::fs::read_to_string(wallet_path()).unwrap();
        let mut file: WalletFile = serde_json::from_str(&json).unwrap();
        file.version = 99;
        std::fs::write(wallet_path(), serde_json::to_string_pretty(&file).unwrap()).unwrap();
        assert!(load_wallet(TEST_PASSWORD).is_err());
    }

    #[test]
    #[serial]
    fn test_save_wallet_different_passwords_give_different_ciphertext() {
        let dir1 = TempDir::new().unwrap();
        setup(&dir1);
        save_wallet(TEST_MNEMONIC, "password1").unwrap();
        let json1 = std::fs::read_to_string(wallet_path()).unwrap();
        let file1: WalletFile = serde_json::from_str(&json1).unwrap();

        let dir2 = TempDir::new().unwrap();
        setup(&dir2);
        save_wallet(TEST_MNEMONIC, "password2").unwrap();
        let json2 = std::fs::read_to_string(wallet_path()).unwrap();
        let file2: WalletFile = serde_json::from_str(&json2).unwrap();

        assert_ne!(file1.encrypted, file2.encrypted);
    }

    #[test]
    #[serial]
    fn test_save_wallet_same_input_gives_different_ciphertext() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        println!("HOME: {:?}", std::env::var("HOME"));
        println!("wallet_path: {:?}", wallet_path());
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let json1 = std::fs::read_to_string(wallet_path()).unwrap();
        let file1: WalletFile = serde_json::from_str(&json1).unwrap();

        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let json2 = std::fs::read_to_string(wallet_path()).unwrap();
        let file2: WalletFile = serde_json::from_str(&json2).unwrap();

        assert_ne!(file1.encrypted, file2.encrypted);
    }

    #[test]
    #[serial]
    fn test_load_wallet_empty_password() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, "").unwrap();
        let data = load_wallet("").unwrap();
        assert_eq!(data.mnemonic, TEST_MNEMONIC);
    }

    #[test]
    #[serial]
    fn test_change_password_success() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let new_pass = "N3w$ecureP@ss99!";
        change_password(TEST_PASSWORD, new_pass).unwrap();
        let data = load_wallet(new_pass).unwrap();
        assert_eq!(data.mnemonic, TEST_MNEMONIC);
    }

    #[test]
    #[serial]
    fn test_change_password_old_password_no_longer_works() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let new_pass = "N3w$ecureP@ss99!";
        change_password(TEST_PASSWORD, new_pass).unwrap();
        assert!(load_wallet(TEST_PASSWORD).is_err());
    }

    #[test]
    #[serial]
    fn test_change_password_wrong_old_password() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        assert!(change_password("wrong_old_pass", "N3w$ecureP@ss99!").is_err());
    }

    #[test]
    #[serial]
    fn test_change_password_weak_new_password() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        assert!(change_password(TEST_PASSWORD, "weak").is_err());
    }

    #[test]
    #[serial]
    fn test_change_password_no_wallet() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        assert!(change_password(TEST_PASSWORD, "N3w$ecureP@ss99!").is_err());
    }

    #[test]
    #[serial]
    fn test_change_password_mnemonic_preserved() {
        let dir = TempDir::new().unwrap();
        setup(&dir);
        save_wallet(TEST_MNEMONIC, TEST_PASSWORD).unwrap();
        let new_pass = "N3w$ecureP@ss99!";
        change_password(TEST_PASSWORD, new_pass).unwrap();
        let data = load_wallet(new_pass).unwrap();
        assert_eq!(data.mnemonic, TEST_MNEMONIC);
    }
}