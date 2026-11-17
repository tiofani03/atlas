use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const ENC_PREFIX: &str = "enc:v1:";

/// Lightweight secret resolution and local encryption helper.
///
/// Supports:
/// - `${ENV_VAR}` and `env:ENV_VAR` references
/// - `enc:v1:<nonce_hex>:<payload_hex>:<mac_hex>` authenticated local encryption
/// - Plaintext fallback for backwards compatibility
pub struct SecretStore;

impl SecretStore {
    /// Resolve a potentially-referenced or encrypted secret value.
    pub fn resolve(raw: &str) -> Result<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("Secret value is empty");
        }

        if let Some(var_name) = Self::parse_env_ref(trimmed) {
            return std::env::var(var_name).with_context(|| {
                format!("Environment variable '{}' referenced by secret is not set", var_name)
            });
        }

        if let Some(rest) = trimmed.strip_prefix(ENC_PREFIX) {
            let key = Self::load_or_create_master_key()?;
            return Self::decrypt_with_key(rest, &key);
        }

        Ok(trimmed.to_string())
    }

    /// Encrypt a plaintext secret into `enc:v1:...` format using the local master key.
    pub fn encrypt(plaintext: &str) -> Result<String> {
        let key = Self::load_or_create_master_key()?;
        Self::encrypt_with_key(plaintext, &key)
    }

    /// Returns true if the value is an environment reference or encrypted payload (not plaintext).
    pub fn is_protected(raw: &str) -> bool {
        let trimmed = raw.trim();
        Self::parse_env_ref(trimmed).is_some() || trimmed.starts_with(ENC_PREFIX)
    }

    /// Redact a secret for safe display in logs or UI responses.
    pub fn redact(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if Self::parse_env_ref(trimmed).is_some() {
            return Some(trimmed.to_string());
        }
        if trimmed.starts_with(ENC_PREFIX) {
            return Some("enc:v1:****".to_string());
        }
        if trimmed.len() <= 4 {
            return Some("****".to_string());
        }
        Some(format!(
            "{}...{}",
            &trimmed[..2],
            &trimmed[trimmed.len() - 2..]
        ))
    }

    /// Set restrictive `0600` file permissions on Unix systems.
    pub fn set_restrictive_permissions(path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| format!("Failed to set 0600 permissions on {:?}", path))?;
        }
        #[cfg(not(unix))]
        {
            let _ = path;
        }
        Ok(())
    }

    fn parse_env_ref(trimmed: &str) -> Option<&str> {
        if trimmed.starts_with("${") && trimmed.ends_with('}') && trimmed.len() > 3 {
            Some(&trimmed[2..trimmed.len() - 1])
        } else {
            trimmed.strip_prefix("env:").filter(|s| !s.is_empty())
        }
    }

    fn default_master_key_path() -> Result<PathBuf> {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| anyhow::anyhow!("HOME environment variable is not set"))?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("atlas")
            .join(".master.key"))
    }

    fn load_or_create_master_key() -> Result<[u8; 32]> {
        if let Ok(env_key) = std::env::var("ATLAS_MASTER_KEY") {
            if !env_key.trim().is_empty() {
                let mut hasher = Sha256::new();
                hasher.update(env_key.as_bytes());
                return Ok(hasher.finalize().into());
            }
        }

        let path = Self::default_master_key_path()?;
        if path.exists() {
            let hex_str = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read master key at {:?}", path))?;
            let bytes = hex_decode(hex_str.trim())?;
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return Ok(key);
            }
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let key = Self::generate_random_32();
        fs::write(&path, hex_encode(&key))
            .with_context(|| format!("Failed to write master key at {:?}", path))?;
        Self::set_restrictive_permissions(&path)?;
        Ok(key)
    }

    fn generate_random_32() -> [u8; 32] {
        // Combine OS-entropy sources (nanosecond timestamp, pid, stack address) hashed with SHA-256.
        let mut hasher = Sha256::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        hasher.update(now.to_le_bytes());
        hasher.update(std::process::id().to_le_bytes());
        let stack_var: u64 = 0x9e37_79b9_7f4a_7c15;
        let ptr = &stack_var as *const u64 as usize;
        hasher.update(ptr.to_le_bytes());
        if let Ok(mut f) = fs::File::open("/dev/urandom") {
            use std::io::Read;
            let mut buf = [0u8; 32];
            if f.read_exact(&mut buf).is_ok() {
                hasher.update(buf);
            }
        }
        hasher.finalize().into()
    }

    pub(crate) fn encrypt_with_key(plaintext: &str, key: &[u8; 32]) -> Result<String> {
        let full_rand = Self::generate_random_32();
        let nonce = &full_rand[..16];
        let pt_bytes = plaintext.as_bytes();
        let ct = xor_keystream(pt_bytes, key, nonce);
        let mac = compute_mac(key, nonce, &ct);
        Ok(format!(
            "{}{}:{}:{}",
            ENC_PREFIX,
            hex_encode(nonce),
            hex_encode(&ct),
            hex_encode(&mac)
        ))
    }

    pub(crate) fn decrypt_with_key(payload: &str, key: &[u8; 32]) -> Result<String> {
        let parts: Vec<&str> = payload.split(':').collect();
        if parts.len() != 3 {
            bail!("Invalid encrypted secret format");
        }
        let nonce = hex_decode(parts[0])?;
        let ct = hex_decode(parts[1])?;
        let mac = hex_decode(parts[2])?;
        let expected_mac = compute_mac(key, &nonce, &ct);
        if mac.len() != expected_mac.len()
            || mac.iter().zip(expected_mac.iter()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) != 0
        {
            bail!("Encrypted secret integrity check failed");
        }
        let pt = xor_keystream(&ct, key, &nonce);
        String::from_utf8(pt).context("Decrypted secret is not valid UTF-8")
    }
}

fn xor_keystream(input: &[u8], key: &[u8; 32], nonce: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for (counter, chunk) in (0_u64..).zip(input.chunks(32)) {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(nonce);
        hasher.update(counter.to_le_bytes());
        let block = hasher.finalize();
        for (b, k) in chunk.iter().zip(block.iter()) {
            out.push(b ^ k);
        }
    }
    out
}

fn compute_mac(key: &[u8; 32], nonce: &[u8], ct: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"atlas-mac-v1");
    hasher.update(key);
    hasher.update(nonce);
    hasher.update(ct);
    hasher.finalize().into()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        bail!("Invalid hex length");
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16)
            .with_context(|| format!("Invalid hex byte at offset {}", i))?;
        out.push(byte);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let secret = "ghp_super_secret_token_123456";
        let encrypted = SecretStore::encrypt_with_key(secret, &key).unwrap();
        assert!(encrypted.starts_with(ENC_PREFIX));
        assert!(SecretStore::is_protected(&encrypted));

        let payload = encrypted.strip_prefix(ENC_PREFIX).unwrap();
        let decrypted = SecretStore::decrypt_with_key(payload, &key).unwrap();
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn test_tampered_ciphertext_fails_mac() {
        let key = [7u8; 32];
        let encrypted = SecretStore::encrypt_with_key("my-secret", &key).unwrap();
        let mut tampered = encrypted.strip_prefix(ENC_PREFIX).unwrap().to_string();
        let last = if tampered.ends_with('0') { '1' } else { '0' };
        tampered.pop();
        tampered.push(last);
        assert!(SecretStore::decrypt_with_key(&tampered, &key).is_err());
    }

    #[test]
    fn test_env_ref_resolution_and_redaction() {
        std::env::set_var("ATLAS_SECRET_TEST_VAR", "val-999");
        assert_eq!(SecretStore::resolve("${ATLAS_SECRET_TEST_VAR}").unwrap(), "val-999");
        assert_eq!(SecretStore::resolve("env:ATLAS_SECRET_TEST_VAR").unwrap(), "val-999");
        assert_eq!(SecretStore::redact("enc:v1:abc:def:ghi").as_deref(), Some("enc:v1:****"));
    }
}
