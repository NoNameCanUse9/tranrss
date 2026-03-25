use magic_crypt::{new_magic_crypt, MagicCryptTrait, MagicCrypt256};
use std::sync::OnceLock;
use std::env;

static CRYPT: OnceLock<MagicCrypt256> = OnceLock::new();

fn get_crypt() -> &'static MagicCrypt256 {
    CRYPT.get_or_init(|| {
        let key = env::var("API_ENCRYPTION_KEY").unwrap_or_else(|_| "default_secure_key_1234567890abcdef".to_string());
        new_magic_crypt!(key, 256)
    })
}

/// 将字符串加密回 Base64
pub fn encrypt(data: &str) -> String {
    if data.is_empty() { return String::new(); }
    get_crypt().encrypt_str_to_base64(data)
}

/// 尝试解密。如果失败则假定原本就是明文（用于兼容旧数据）
pub fn decrypt_safe(encrypted: &str) -> String {
    if encrypted.is_empty() { return String::new(); }
    
    match get_crypt().decrypt_base64_to_string(encrypted) {
        Ok(plain) => plain,
        Err(_) => encrypted.to_string(), // 可能是明文或解密密钥不匹配
    }
}
