use crate::privacy::encryption::{decrypt, encrypt};
use std::fs;
use std::path::{Path, PathBuf};

const ENCRYPTED_EXT: &str = "enc";

/// Encrypt a file in-place: reads plaintext, writes ciphertext with .enc extension.
/// Returns the path to the encrypted file.
pub fn encrypt_file(path: &Path, key: &[u8; 32]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let plaintext = fs::read(path)?;
    let ciphertext = encrypt(key, &plaintext).map_err(|e| format!("Encryption failed: {:?}", e))?;

    let enc_path = path.with_extension(format!(
        "{}.{}",
        path.extension().unwrap_or_default().to_str().unwrap_or(""),
        ENCRYPTED_EXT
    ));

    fs::write(&enc_path, &ciphertext)?;

    // Remove the original plaintext file
    fs::remove_file(path)?;

    log::debug!(
        "Encrypted {} → {} ({} → {} bytes)",
        path.display(),
        enc_path.display(),
        plaintext.len(),
        ciphertext.len()
    );

    Ok(enc_path)
}

/// Decrypt a .enc file and return the plaintext bytes.
pub fn decrypt_file(enc_path: &Path, key: &[u8; 32]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let ciphertext = fs::read(enc_path)?;
    let plaintext = decrypt(key, &ciphertext).map_err(|e| format!("Decryption failed: {:?}", e))?;
    Ok(plaintext)
}

/// Decrypt a .enc file to a temporary file for reading (e.g., video playback).
/// Returns the path to the temp file which the caller must clean up.
pub fn decrypt_to_temp(
    enc_path: &Path,
    key: &[u8; 32],
) -> Result<tempfile::NamedTempFile, Box<dyn std::error::Error>> {
    let plaintext = decrypt_file(enc_path, key)?;

    // Determine original extension by stripping .enc
    let stem = enc_path.file_stem().unwrap_or_default().to_str().unwrap_or("");
    let orig_ext = Path::new(stem)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("mkv");

    let tmp = tempfile::Builder::new()
        .suffix(&format!(".{}", orig_ext))
        .tempfile()?;

    fs::write(tmp.path(), &plaintext)?;
    Ok(tmp)
}

/// Encrypt all unencrypted segment files (.mkv) in a directory.
/// Skips files that already have .enc extension.
/// Returns the number of files encrypted.
pub fn encrypt_segments_in_dir(
    dir: &Path,
    key: &[u8; 32],
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;

    for entry in fs::read_dir(dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(true, |ext| ext == ENCRYPTED_EXT) {
            continue; // Skip already encrypted or extensionless
        }
        if path.extension().map_or(false, |ext| ext == "mkv" || ext == "jsonl") {
            encrypt_file(&path, key)?;
            count += 1;
        }
    }

    if count > 0 {
        log::info!("Encrypted {} segment files in {:?}", count, dir);
    }

    Ok(count)
}

/// Check if a segment file is encrypted (has .enc in its extension chain).
pub fn is_encrypted(path: &Path) -> bool {
    path.to_str()
        .map_or(false, |s| s.contains(&format!(".{}", ENCRYPTED_EXT)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_key() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn test_encrypt_decrypt_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.mkv");
        let original_data = b"This is video data for testing encryption";
        fs::write(&file_path, original_data).unwrap();

        // Encrypt
        let enc_path = encrypt_file(&file_path, &test_key()).unwrap();
        assert!(enc_path.exists());
        assert!(!file_path.exists()); // Original should be deleted
        assert!(is_encrypted(&enc_path));

        // Encrypted data should be different from original
        let enc_data = fs::read(&enc_path).unwrap();
        assert_ne!(enc_data, original_data);

        // Decrypt
        let decrypted = decrypt_file(&enc_path, &test_key()).unwrap();
        assert_eq!(decrypted, original_data);
    }

    #[test]
    fn test_decrypt_to_temp() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("segment.mkv");
        fs::write(&file_path, b"video data").unwrap();

        let enc_path = encrypt_file(&file_path, &test_key()).unwrap();
        let tmp = decrypt_to_temp(&enc_path, &test_key()).unwrap();

        assert_eq!(fs::read(tmp.path()).unwrap(), b"video data");
        assert!(tmp.path().to_str().unwrap().contains(".mkv"));
    }

    #[test]
    fn test_encrypt_segments_in_dir() {
        let dir = tempfile::tempdir().unwrap();

        // Create some segment files
        fs::write(dir.path().join("seg_1.mkv"), b"video1").unwrap();
        fs::write(dir.path().join("seg_1.jsonl"), b"meta1").unwrap();
        fs::write(dir.path().join("seg_2.mkv"), b"video2").unwrap();
        fs::write(dir.path().join("other.txt"), b"ignore").unwrap();

        let count = encrypt_segments_in_dir(dir.path(), &test_key()).unwrap();
        assert_eq!(count, 3); // 2 mkv + 1 jsonl

        // Original files should be gone
        assert!(!dir.path().join("seg_1.mkv").exists());
        assert!(!dir.path().join("seg_1.jsonl").exists());

        // .txt should be untouched
        assert!(dir.path().join("other.txt").exists());
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("secret.mkv");
        fs::write(&file_path, b"secret data").unwrap();

        let enc_path = encrypt_file(&file_path, &test_key()).unwrap();
        let wrong_key = [99u8; 32];
        assert!(decrypt_file(&enc_path, &wrong_key).is_err());
    }

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted(Path::new("segment_123.mkv.enc")));
        assert!(is_encrypted(Path::new("data.jsonl.enc")));
        assert!(!is_encrypted(Path::new("segment_123.mkv")));
        assert!(!is_encrypted(Path::new("data.jsonl")));
    }
}
