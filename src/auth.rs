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

/// token 有效期 30 天（用户 token 与管理员 token 共用）
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

/// 去掉易混淆字符的可读随机串，用于邀请码与初始管理密码。
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

/// 普通用户鉴权：只认 TOKENS -> USERS，管理员会话不在这两张表里，天然无法访问聊天接口。
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

/// 普通用户提取器：所有聊天 / 文件 / 用户类 API 都用它。
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

/// 管理员提取器：只认 ADMIN_TOKENS 表，不查 USERS 表。
/// 管理员是服务器级身份，没有用户名、不是聊天用户，用户 token 也进不了管理接口。
pub struct AdminAuth;

#[async_trait]
impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts);
        if token.is_empty() {
            return Err(ApiError::unauthorized("缺少 token"));
        }
        let expires = db::get_text(state.db.as_ref(), db::ADMIN_TOKENS, token)?
            .ok_or_else(|| ApiError::unauthorized("登录已失效，请重新登录"))?;
        let expires_at: i64 = expires
            .parse()
            .map_err(|_| ApiError::unauthorized("登录状态异常，请重新登录"))?;
        if expires_at <= now_ms() {
            return Err(ApiError::unauthorized("登录已过期，请重新登录"));
        }
        Ok(Self)
    }
}

/// 首次启动生成初始管理密码：随机密码只打印到日志，哈希写入 ADMIN 表，不占用任何用户名。
/// 若事先执行过 `passwd`（ADMIN 表已有哈希），这里直接跳过。
pub fn ensure_admin(db: &redb::Database) -> ApiResult<()> {
    if db::get_text(db, db::ADMIN, db::ADMIN_PASSWORD_KEY)?.is_some() {
        return Ok(());
    }
    let password = random_code();
    db::put_text(
        db,
        db::ADMIN,
        db::ADMIN_PASSWORD_KEY,
        &hash_password(&password)?,
    )?;

    tracing::warn!("=====================================================");
    tracing::warn!("  Chatware 初始管理密码已生成");
    tracing::warn!("  密码  : {password}");
    tracing::warn!("  在 /admin 输入此密码即可登录（无需用户名）");
    tracing::warn!(
        "  可用 passwd 子命令自行设定： ./chatware passwd -d <数据目录> --password <新密码>"
    );
    tracing::warn!("  （该密码只在此日志出现一次）");
    tracing::warn!("=====================================================");
    Ok(())
}

/// 离线设置 / 重置管理密码（passwd 子命令调用）：写哈希 + 吊销全部管理员会话。
pub fn set_admin_password(db: &redb::Database, password: &str) -> ApiResult<()> {
    validate_password(password)?;
    db::write_admin_password(db, &hash_password(password)?)
}

/// 管理面板在线改密：先校验当前管理密码，成功后吊销全部管理员会话（含当前）。
pub fn change_admin_password(
    db: &redb::Database,
    current_password: &str,
    new_password: &str,
) -> ApiResult<()> {
    validate_password(new_password)?;
    let hash = db::get_text(db, db::ADMIN, db::ADMIN_PASSWORD_KEY)?
        .ok_or_else(|| ApiError::internal("管理密码尚未初始化"))?;
    if !verify_password(current_password, &hash) {
        return Err(ApiError::bad_request("当前管理密码不正确"));
    }
    db::write_admin_password(db, &hash_password(new_password)?)
}

/// 管理面板登录：只校验密码，签发管理员会话 token。
pub fn admin_login(state: &AppState, password: &str) -> ApiResult<String> {
    let hash = db::get_text(state.db.as_ref(), db::ADMIN, db::ADMIN_PASSWORD_KEY)?
        .ok_or_else(|| ApiError::internal("管理密码尚未初始化"))?;
    if !verify_password(password, &hash) {
        return Err(ApiError::unauthorized("密码错误"));
    }
    let token = random_token();
    let expires_at = now_ms().saturating_add(TOKEN_TTL_MS);
    db::put_text(
        state.db.as_ref(),
        db::ADMIN_TOKENS,
        &token,
        &expires_at.to_string(),
    )?;
    Ok(token)
}
