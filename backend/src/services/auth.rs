use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, encode};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// JWT Secret 全局单例 —— 优先从环境变量加载，其次从数据库加载，最后从文件/自动生成加载
pub static JWT_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

/// 手动初始化 JWT Secret (用于从数据库加载)
pub fn init_jwt_secret(secret: Vec<u8>) -> Result<(), Vec<u8>> {
    JWT_SECRET.set(secret)
}

pub fn get_jwt_secret() -> &'static [u8] {
    JWT_SECRET.get_or_init(|| {
        // 1. 优先尝试从环境变量读取（推荐 Docker 部署方式）
        if let Ok(val) = std::env::var("JWT_SECRET") {
            if let Ok(bytes) = hex::decode(val.trim()) {
                if bytes.len() >= 32 {
                    tracing::info!("JWT secret 已从环境变量加载");
                    return bytes;
                }
            }
            tracing::warn!("环境变量 JWT_SECRET 无效（需为 64 字符 hex）");
        }

        // 2. 如果没有任何预设（环境变量或数据库初始化），则生成一个临时的
        // 注意：生产环境下 main.rs 会负责从数据库加载并通过 init_jwt_secret 初始化
        // 如果运行到这一步，说明是真正意义上的“首次启动”或“数据库刚被删”
        let mut secret = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        tracing::info!("已生成新的随机 JWT secret");
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
        let token = if let Some(t) = auth_header.strip_prefix("Bearer ") {
            t.trim()
        } else if let Some(t) = auth_header.strip_prefix("GoogleLogin auth=") {
            t.trim_matches('"').trim()
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
    // 3 天过期时间
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(3))
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
