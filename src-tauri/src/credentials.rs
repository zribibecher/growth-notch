//! Encrypted local storage for the YouTube API key and channel ID.
//!
//! `vault.enc` holds a JSON map encrypted with ChaCha20-Poly1305 (12-byte nonce, then ciphertext).
//! The 32-byte key sits next to it in `.vault.key`. Both files are mode 0600 inside a 0700
//! directory. That keeps the secrets out of plaintext files, but it does not protect against
//! malware running as the same user, and it is not the macOS Keychain.

use crate::security::{
    validate_api_key_format, validate_safe_identifier, CredentialProvider, RedactedSecret,
};
use chacha20poly1305::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use zeroize::Zeroizing;

const KEY_FILE_NAME: &str = ".vault.key";
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const MAX_SECRET_LEN: usize = 4096;

type Entries = HashMap<CredentialProvider, RedactedSecret>;

/// What the UI is told about stored credentials: presence only, never the values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub youtube_configured: bool,
    pub youtube_channel_configured: bool,
}

/// The vault. With a file it persists every change; without one (tests) it only lives in memory.
pub struct FileVault {
    vault_file: Option<PathBuf>,
    entries: Mutex<Entries>,
}

impl FileVault {
    pub fn in_memory() -> Self {
        Self {
            vault_file: None,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Opens the vault stored at `vault_file`, reading it if it exists and can be decrypted.
    /// A missing, truncated or tampered file simply yields an empty vault.
    pub fn open(vault_file: PathBuf) -> Self {
        let entries = load_entries(&vault_file).unwrap_or_default();
        Self {
            vault_file: Some(vault_file),
            entries: Mutex::new(entries),
        }
    }

    pub fn set_secret(&self, provider: CredentialProvider, secret: &str) -> Result<(), String> {
        let secret = Zeroizing::new(secret.trim().to_string());
        if secret.is_empty() {
            return Err("Credential value cannot be empty".into());
        }
        if secret.len() > MAX_SECRET_LEN {
            return Err(format!(
                "Credential value exceeds maximum allowed length ({MAX_SECRET_LEN} bytes)"
            ));
        }
        match provider {
            CredentialProvider::YoutubeChannelId => validate_safe_identifier(&secret, "Channel ID")
                .map_err(|_| "That is not a valid channel ID. It starts with UC.".to_string())?,
            CredentialProvider::YoutubeApiKey => {
                validate_api_key_format(&secret).map_err(|_| {
                    "That is not a valid API key. Copy it again from Google Cloud.".to_string()
                })?
            }
        };

        let mut entries = self.lock()?;
        entries.insert(provider, RedactedSecret::new(secret.as_str()));
        self.persist(&entries)
    }

    pub fn get_secret(
        &self,
        provider: CredentialProvider,
    ) -> Result<Option<RedactedSecret>, String> {
        Ok(self.lock()?.get(&provider).cloned())
    }

    pub fn delete_secret(&self, provider: CredentialProvider) -> Result<(), String> {
        let mut entries = self.lock()?;
        entries.remove(&provider);
        self.persist(&entries)
    }

    pub fn status(&self) -> Result<CredentialStatus, String> {
        let entries = self.lock()?;
        Ok(CredentialStatus {
            youtube_configured: entries.contains_key(&CredentialProvider::YoutubeApiKey),
            youtube_channel_configured: entries.contains_key(&CredentialProvider::YoutubeChannelId),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Entries>, String> {
        self.entries
            .lock()
            .map_err(|_| "Credential store is unavailable".to_string())
    }

    fn persist(&self, entries: &Entries) -> Result<(), String> {
        let Some(vault_file) = &self.vault_file else {
            return Ok(());
        };
        if let Some(dir) = vault_file.parent() {
            create_private_dir(dir)?;
        }

        let key = load_or_create_key(&key_path(vault_file))?;
        let plain: HashMap<&str, &str> = entries
            .iter()
            .map(|(provider, secret)| (provider.as_str(), secret.expose_secret()))
            .collect();
        let plaintext = Zeroizing::new(
            serde_json::to_vec(&plain)
                .map_err(|_| "Failed to serialize credentials".to_string())?,
        );

        let mut nonce = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| "The vault key is invalid".to_string())?
            .encrypt(&Nonce::from(nonce), plaintext.as_slice())
            .map_err(|_| "Failed to encrypt credentials".to_string())?;

        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);
        write_private_file(vault_file, &payload)
    }
}

fn key_path(vault_file: &Path) -> PathBuf {
    vault_file.with_file_name(KEY_FILE_NAME)
}

fn load_entries(vault_file: &Path) -> Option<Entries> {
    let key = Zeroizing::new(std::fs::read(key_path(vault_file)).ok()?);
    let raw = std::fs::read(vault_file).ok()?;
    if key.len() != KEY_LEN || raw.len() <= NONCE_LEN {
        return None;
    }

    let (nonce, ciphertext) = raw.split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().ok()?;
    let plaintext = Zeroizing::new(
        ChaCha20Poly1305::new_from_slice(&key)
            .ok()?
            .decrypt(&Nonce::from(nonce), ciphertext)
            .ok()?,
    );
    let stored: HashMap<String, String> = serde_json::from_slice(&plaintext).ok()?;

    let entries = [
        CredentialProvider::YoutubeApiKey,
        CredentialProvider::YoutubeChannelId,
    ]
    .into_iter()
    .filter_map(|provider| {
        let value = stored.get(provider.as_str())?;
        (!value.trim().is_empty()).then(|| (provider, RedactedSecret::new(value.as_str())))
    })
    .collect();
    Some(entries)
}

/// Reads the vault key, or creates a new random one if there is none (or it is the wrong size).
fn load_or_create_key(path: &Path) -> Result<Zeroizing<Vec<u8>>, String> {
    if let Ok(existing) = std::fs::read(path) {
        if existing.len() == KEY_LEN {
            return Ok(Zeroizing::new(existing));
        }
    }
    let mut key = Zeroizing::new(vec![0u8; KEY_LEN]);
    OsRng.fill_bytes(key.as_mut_slice());
    write_private_file(path, key.as_slice())?;
    Ok(key)
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| format!("Failed to create the app data directory: {e}"))?;
    // A directory that already existed keeps its old mode, so tighten it explicitly.
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    Ok(())
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create the app data directory: {e}"))
}

/// Writes to a sibling temporary file and renames it into place, so a crash mid-write can never
/// leave a truncated file behind.
fn write_private_file(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&tmp)
        .map_err(|e| format!("Failed to open a secure file: {e}"))?;
    file.write_all(data)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Failed to write a secure file: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Failed to replace a secure file: {e}"))
}
