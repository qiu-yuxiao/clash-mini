use crate::utils::dirs::get_encryption_key;
use aes_gcm::{
    Aes256Gcm, Key,
    aead::{Aead as _, KeyInit as _},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::Cell;

const NONCE_LENGTH: usize = 12;

// 使用 thread_local 而非 tokio::task_local，因为加密标志只在同步的
// serde 序列化/反序列化期间使用，不跨 .await 点。
// thread_local 避免了 with_encryption 的 async 传染。
thread_local! {
    static ENCRYPTION_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Encrypt data
#[allow(deprecated)]
pub fn encrypt_data(data: &str) -> Result<String, Box<dyn std::error::Error>> {
    let encryption_key = get_encryption_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(key);

    // Generate random nonce
    let mut nonce = vec![0u8; NONCE_LENGTH];
    getrandom::fill(&mut nonce)?;

    // Encrypt data
    let ciphertext = cipher
        .encrypt(nonce.as_slice().into(), data.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;

    // Concatenate nonce and ciphertext and encode them in base64
    let mut combined = nonce;
    combined.extend(ciphertext);
    Ok(STANDARD.encode(combined))
}

/// Decrypt data
#[allow(deprecated)]
pub fn decrypt_data(encrypted: &str) -> Result<String, Box<dyn std::error::Error>> {
    let encryption_key = get_encryption_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(key);
    // Decode from base64
    let data = STANDARD.decode(encrypted)?;
    if data.len() < NONCE_LENGTH {
        return Err("Invalid encrypted data".into());
    }

    // Separate nonce and ciphertext
    let (nonce, ciphertext) = data.split_at(NONCE_LENGTH);

    // Decrypt data
    let plaintext = cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|e| format!("Decryption failed: {e}"))?;

    String::from_utf8(plaintext).map_err(|e| e.into())
}

/// Serialize encrypted function
pub fn serialize_encrypted<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    if is_encryption_active() {
        let json = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        let encrypted = encrypt_data(&json).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&encrypted)
    } else {
        value.serialize(serializer)
    }
}

/// Deserialize decrypted function
pub fn deserialize_encrypted<'a, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: for<'de> Deserialize<'de> + Default,
    D: Deserializer<'a>,
{
    if is_encryption_active() {
        let encrypted_opt: Option<String> = Option::deserialize(deserializer)?;

        match encrypted_opt {
            Some(encrypted) if !encrypted.is_empty() => {
                let decrypted_string = decrypt_data(&encrypted).map_err(serde::de::Error::custom)?;
                serde_json::from_str(&decrypted_string).map_err(serde::de::Error::custom)
            }
            _ => Ok(T::default()),
        }
    } else {
        T::deserialize(deserializer)
    }
}

/// 在加密上下文中执行闭包。
///
/// 同步实现：设置 thread_local 标志后执行闭包，完毕后恢复。
/// 闭包内执行的 serde 序列化/反序列化通过 is_encryption_active() 读取此标志。
pub fn with_encryption<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    ENCRYPTION_ACTIVE.with(|c| {
        let old = c.get();
        c.set(true);
        let result = f();
        c.set(old);
        result
    })
}

fn is_encryption_active() -> bool {
    ENCRYPTION_ACTIVE.try_with(|c| c.get()).unwrap_or(false)
}
