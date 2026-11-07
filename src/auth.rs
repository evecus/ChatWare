use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::{TokenRecord, UserRecord};
use crate::state::AppState;
use argon2::password_hash::{
    rand_core::OsRng as HashOsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use rand::rngs::OsRng as TokenOsRng;
use rand::{Rng, RngCore};
use std::path::Path;

/// token 有效期 30 天
const TOKEN_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut HashOsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(ApiError::internal)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn random_token() -> String {
    let mut buf = [0u8; 32];
    TokenOsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

/// 去掉易混淆字符的可读随机串，用于邀请码与初始管理员密码。
pub fn random_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

pub fn validate_username(name: &str) -> ApiResult<()> {
    let len = name.chars().count();
    if !(2..=24).contains(&len) {
        return Err(ApiError::bad_request("用户名长度需为 2~24 个字符"));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ApiError::bad_request(
            "用户名只能包含字母、数字、下划线或连字符",
        ));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> ApiResult<()> {
    let len = password.chars().count();
    if !(8..=128).contains(&len) {
        return Err(ApiError::bad_request("密码长度需为 8~128 个字符"));
    }
    Ok(())
}

pub fn new_token_record(username: &str) -> TokenRecord {
    let now = now_ms();
    TokenRecord {
        username: username.to_string(),
        created_at: now,
        expires_at: now.saturating_add(TOKEN_TTL_MS),
    }
}

pub fn user_by_token(state: &AppState, token: &str) -> ApiResult<UserRecord> {
    if token.is_empty() {
        return Err(ApiError::unauthorized("缺少 token"));
    }
    let record: Option<TokenRecord> = db::get_json(state.db.as_ref(), db::TOKENS, token)?;
    let record = record.ok_or_else(|| ApiError::unauthorized("token 无效或已被吊销"))?;
    if record.expires_at <= now_ms() {
        return Err(ApiError::unauthorized("token 已过期，请重新登录"));
    }
    let user: Option<UserRecord> = db::get_json(state.db.as_ref(), db::USERS, &record.username)?;
    let user = user.ok_or_else(|| ApiError::unauthorized("账号不存在"))?;
    if user.disabled {
        return Err(ApiError::forbidden("账号已被管理员禁用"));
    }
    Ok(user)
}

fn bearer_token(parts: &Parts) -> &str {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .trim()
}

/// 普通用户鉴权：所有登录后才可访问的 API 都用这个提取器。
pub struct AuthUser(pub UserRecord);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = user_by_token(state, bearer_token(parts))?;
        Ok(Self(user))
    }
}

/// 管理员鉴权：初始密码未修改之前，除改密接口外一律拒绝。
pub struct AdminUser(pub UserRecord);

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?.0;
        if !user.is_admin() {
            return Err(ApiError::forbidden("需要管理员权限"));
        }
        if user.must_change_password {
            return Err(ApiError::forbidden("请先修改初始管理员密码"));
        }
        Ok(Self(user))
    }
}

/// 首次启动创建管理员：随机密码打印到日志并写入 data/admin.txt，且强制首次登录改密。
pub fn ensure_admin(db: &redb::Database, data_dir: &Path) -> ApiResult<()> {
    let users: Vec<UserRecord> = db::all_json(db, db::USERS)?;
    if users.iter().any(UserRecord::is_admin) {
        return Ok(());
    }
    let password = random_code();
    let record = UserRecord {
        username: "admin".to_string(),
        password_hash: hash_password(&password)?,
        role: "admin".to_string(),
        disabled: false,
        must_change_password: true,
        created_at: now_ms(),
    };
    db::put_json(db, db::USERS, "admin", &record)?;

    let path = data_dir.join("admin.txt");
    let content = format!(
        "Chatware 初始管理员\n用户名: admin\n密码: {password}\n首次登录 /admin 后必须修改密码。\n"
    );
    std::fs::write(&path, &content).map_err(ApiError::internal)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    tracing::warn!("=====================================================");
    tracing::warn!("  Chatware 初始管理员已创建");
    tracing::warn!("  用户名: admin");
    tracing::warn!("  密码  : {password}");
    tracing::warn!("  首次登录 /admin 后必须修改密码");
    tracing::warn!("  同时已写入 {}", path.display());
    tracing::warn!("=====================================================");
    Ok(())
}
