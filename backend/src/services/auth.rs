use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, encode};
use serde::{Deserialize, Serialize};

// 保持 &[u8; 12] 自然类型，传入函数时会自动 coerce 为 &[u8]，无需 const 中做切片
const JWT_SECRET: &[u8] = b"hqt384058029";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 存储 user_id
    pub exp: usize,  // 过期时间
    pub iat: usize,  // 签发时间
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
    // 这里的 Rejection 必须与 axum 要求的类型一致
    type Rejection = (StatusCode, String);

    // 注意：这里显式使用 std::result::Result，因为你要返回自定义的 Rejection，
    // 而不是 anyhow::Result (它只接受一个泛型参数)。
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
    // 100 年过期时间，仅用于测试
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
        &EncodingKey::from_secret(JWT_SECRET),
    )?;

    Ok(token)
}

pub fn decode_token(token: &str) -> Result<Claims> {
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    Ok(token_data.claims)
}
