use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, encode};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, sync::OnceLock};

/// JWT Secret 全局单例 —— 首次调用时从文件加载，文件不存在则自动生成并写入
static JWT_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

const SECRET_FILE: &str = "../jwt_secret.key";

fn get_jwt_secret() -> &'static [u8] {
    JWT_SECRET.get_or_init(|| {
        let path = Path::new(SECRET_FILE);
        if path.exists() {
            // 读取已有的 secret（十六进制文本）
            match fs::read_to_string(path) {
                Ok(hex) => {
                    let hex = hex.trim();
                    if let Ok(bytes) = hex::decode(hex) {
                        if bytes.len() >= 32 {
                            tracing::info!("JWT secret 已从 {} 加载", SECRET_FILE);
                            return bytes;
                        }
                    }
                    tracing::warn!("jwt_secret.key 文件内容无效，将重新生成");
                }
                Err(e) => tracing::warn!("读取 jwt_secret.key 失败: {}，将重新生成", e),
            }
        }

        // 生成 32 字节随机 secret
        let mut secret = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        let hex_str = hex::encode(&secret);

        if let Err(e) = fs::write(path, &hex_str) {
            tracing::error!("无法写入 jwt_secret.key: {}，本次使用内存中生成的密钥", e);
        } else {
            tracing::info!("已生成新的 JWT secret 并保存至 {}", SECRET_FILE);
        }

        secret
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // 存储 user_id
    pub exp: usize,     // 过期时间
    pub iat: usize,     // 签发时间
    pub username: String,
}

/// 用于在 Handler 中直接提取用户信息
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
}

// --- 核心：为 AuthUser 实现 Axum 提取器 ---
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        // 1. 获取 Authorization Header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Missing Authorization Header".to_string(),
            ))?;

        // 2. 检查并截取 Token
        let token = if auth_header.starts_with("Bearer ") {
            &auth_header[7..]
        } else if auth_header.starts_with("GoogleLogin auth=") {
            &auth_header[17..]
        } else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization Format".to_string(),
            ));
        };

        // 3. 调用下面的 decode_token
        let claims = decode_token(token)
            .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid Token: {}", e)))?;

        // 4. 将 Claims 转换为 AuthUser
        let user_id = claims.sub.parse::<i64>().unwrap_or(0);

        Ok(AuthUser {
            user_id,
            username: claims.username,
        })
    }
}

// --- 工具函数 ---

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(anyhow::Error::msg)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub fn create_token(user_id: i64, username: &str) -> Result<String> {
    // 100 年过期时间
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(365 * 100))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration as usize,
        iat: Utc::now().timestamp() as usize,
        username: username.to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret()),
    )?;

    Ok(token)
}

pub fn decode_token(token: &str) -> Result<Claims> {
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_jwt_secret()),
        &Validation::default(),
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    Ok(token_data.claims)
}
