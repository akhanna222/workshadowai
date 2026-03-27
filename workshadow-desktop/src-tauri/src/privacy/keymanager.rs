use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Manages the encryption key for data at rest.
/// In production, the key would be stored in the OS keychain
/// (macOS Keychain / Windows DPAPI). For now, we derive a key
/// from a local seed file, which is a stepping stone to keychain integration.
pub struct KeyManager {
    key: [u8; 32],
}

impl KeyManager {
    /// Load or create the encryption key.
    pub fn new(data_dir: &Path) -> Self {
        let seed_path = data_dir.join(".keyfile");
        let key = match Self::load_seed(&seed_path) {
            Some(seed) => Self::derive_key(&seed),
            None => {
                let seed = Self::generate_seed();
                Self::save_seed(&seed_path, &seed);
                Self::derive_key(&seed)
            }
        };

        Self { key }
    }

    /// Get the 256-bit encryption key.
    pub fn key(&self) -> &[u8; 32] {
        &self.key
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

    fn load_seed(path: &Path) -> Option<Vec<u8>> {
        fs::read(path).ok()
    }

    fn save_seed(path: &Path, seed: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        // Write with restricted permissions
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
    fn test_key_manager_persists() {
        let dir = tempfile::tempdir().unwrap();
        let km1 = KeyManager::new(dir.path());
        let km2 = KeyManager::new(dir.path());
        assert_eq!(km1.key(), km2.key());
    }

    #[test]
    fn test_key_manager_creates_keyfile() {
        let dir = tempfile::tempdir().unwrap();
        let _km = KeyManager::new(dir.path());
        assert!(dir.path().join(".keyfile").exists());
    }
}
