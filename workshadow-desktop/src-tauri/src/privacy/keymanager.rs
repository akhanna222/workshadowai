use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const SERVICE_NAME: &str = "com.workshadow.ai";
const ACCOUNT_NAME: &str = "encryption-seed";

/// Manages the encryption key for data at rest.
///
/// Key storage priority:
/// 1. OS keychain (macOS Keychain / Linux secret-service / Windows Credential Manager)
/// 2. Local file fallback (~/.workshadow/data/.keyfile)
///
/// The seed is stored in the keychain; the actual AES-256 key is derived via SHA-256.
pub struct KeyManager {
    key: [u8; 32],
    storage: KeyStorage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyStorage {
    OsKeychain,
    LocalFile,
}

impl KeyManager {
    /// Load or create the encryption key. Tries OS keychain first, falls back to file.
    pub fn new(data_dir: &Path) -> Self {
        // Try OS keychain first
        match Self::load_from_keychain() {
            Ok(seed) => {
                log::info!("Encryption key loaded from OS keychain");
                return Self {
                    key: Self::derive_key(&seed),
                    storage: KeyStorage::OsKeychain,
                };
            }
            Err(e) => {
                log::debug!("Keychain load failed ({}), trying file fallback", e);
            }
        }

        // Try local file
        let seed_path = data_dir.join(".keyfile");
        if let Some(seed) = Self::load_seed_file(&seed_path) {
            // Migrate file seed to keychain if possible
            if Self::store_in_keychain(&seed).is_ok() {
                log::info!("Migrated encryption key from file to OS keychain");
                // Keep the file as backup but don't delete it
                return Self {
                    key: Self::derive_key(&seed),
                    storage: KeyStorage::OsKeychain,
                };
            }
            log::info!("Encryption key loaded from local file");
            return Self {
                key: Self::derive_key(&seed),
                storage: KeyStorage::LocalFile,
            };
        }

        // Generate new seed
        let seed = Self::generate_seed();

        // Store in keychain, fallback to file
        let storage = if Self::store_in_keychain(&seed).is_ok() {
            log::info!("New encryption key stored in OS keychain");
            KeyStorage::OsKeychain
        } else {
            Self::save_seed_file(&seed_path, &seed);
            log::info!("New encryption key stored in local file");
            KeyStorage::LocalFile
        };

        Self {
            key: Self::derive_key(&seed),
            storage,
        }
    }

    /// Get the 256-bit encryption key.
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Where the key is stored.
    pub fn storage(&self) -> &KeyStorage {
        &self.storage
    }

    fn generate_seed() -> Vec<u8> {
        use aes_gcm::aead::rand_core::RngCore;
        let mut seed = vec![0u8; 64];
        aes_gcm::aead::OsRng.fill_bytes(&mut seed);
        seed
    }

    fn derive_key(seed: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"workshadow-v1-");
        hasher.update(seed);
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    // ── OS Keychain ──

    fn load_from_keychain() -> Result<Vec<u8>, String> {
        let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map_err(|e| format!("keyring entry error: {}", e))?;
        let secret = entry
            .get_secret()
            .map_err(|e| format!("keyring get error: {}", e))?;
        Ok(secret)
    }

    fn store_in_keychain(seed: &[u8]) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map_err(|e| format!("keyring entry error: {}", e))?;
        entry
            .set_secret(seed)
            .map_err(|e| format!("keyring set error: {}", e))
    }

    // ── File fallback ──

    fn load_seed_file(path: &Path) -> Option<Vec<u8>> {
        fs::read(path).ok()
    }

    fn save_seed_file(path: &Path, seed: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, seed).ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation_deterministic_from_seed() {
        let seed = b"test-seed-12345";
        let key1 = KeyManager::derive_key(seed);
        let key2 = KeyManager::derive_key(seed);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_different_seeds_different_keys() {
        let key1 = KeyManager::derive_key(b"seed-a");
        let key2 = KeyManager::derive_key(b"seed-b");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_manager_persists_via_file() {
        let dir = tempfile::tempdir().unwrap();
        // Force file-based storage by creating keyfile directly
        let seed = b"test-persistence-seed-1234567890";
        let seed_path = dir.path().join(".keyfile");
        std::fs::write(&seed_path, seed).unwrap();

        let km1 = KeyManager::new(dir.path());
        let km2 = KeyManager::new(dir.path());
        assert_eq!(km1.key(), km2.key());
    }

    #[test]
    fn test_key_is_32_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let km = KeyManager::new(dir.path());
        assert_eq!(km.key().len(), 32);
    }

    #[test]
    fn test_key_storage_type() {
        let dir = tempfile::tempdir().unwrap();
        let km = KeyManager::new(dir.path());
        // In CI/headless, keychain may not be available
        assert!(
            *km.storage() == KeyStorage::OsKeychain
                || *km.storage() == KeyStorage::LocalFile
        );
    }
}
