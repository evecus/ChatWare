use crate::auth::{
    hash_password, new_token_record, now_ms, random_token, validate_password, validate_username,
    verify_password, AuthUser,
};
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::{
    ChangePasswordReq, InviteRecord, LoginReq, LoginResp, RegisterReq, UserRecord, UserView,
};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use redb::ReadableTable;
use serde_json::{json, Value};

/// 注册：邀请码核销与账号创建放在同一个写事务里，避免并发超发。
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<Value>> {
    if !state.config_snapshot().allow_registration {
        return Err(ApiError::forbidden("管理员已关闭注册"));
    }
    validate_username(&req.username)?;
    validate_password(&req.password)?;
    let code = req.invite_code.trim().to_uppercase();
    if code.is_empty() {
        return Err(ApiError::bad_request("请输入邀请码"));
    }
    if db::get_json::<UserRecord>(state.db.as_ref(), db::USERS, &req.username)?.is_some() {
        return Err(ApiError::conflict("用户名已存在"));
    }

    let password_hash = hash_password(&req.password)?;
    let record = UserRecord {
        username: req.username.clone(),
        password_hash,
        role: "user".to_string(),
        disabled: false,
        must_change_password: false,
        created_at: now_ms(),
    };
    let user_bytes = serde_json::to_vec(&record).map_err(ApiError::internal)?;

    let txn = state.db.begin_write().map_err(ApiError::internal)?;
    {
        let mut users = txn.open_table(db::USERS).map_err(ApiError::internal)?;
        let mut invites = txn.open_table(db::INVITES).map_err(ApiError::internal)?;
        let mut invite: InviteRecord = {
            let guard = invites
                .get(code.as_str())
                .map_err(ApiError::internal)?
                .ok_or_else(|| ApiError::bad_request("邀请码无效"))?;
            serde_json::from_slice(guard.value()).map_err(ApiError::internal)?
        };
        if invite.used >= invite.max_uses {
            return Err(ApiError::bad_request("邀请码使用次数已用尽"));
        }
        if invite.expires_at != 0 && invite.expires_at <= now_ms() {
            return Err(ApiError::bad_request("邀请码已过期"));
        }
        invite.used = invite.used.saturating_add(1);
        let invite_bytes = serde_json::to_vec(&invite).map_err(ApiError::internal)?;
        invites
            .insert(code.as_str(), invite_bytes.as_slice())
            .map_err(ApiError::internal)?;
        users
            .insert(req.username.as_str(), user_bytes.as_slice())
            .map_err(ApiError::internal)?;
    }
    txn.commit().map_err(ApiError::internal)?;

    tracing::info!(username = %req.username, "新用户注册");
    Ok(Json(json!({ "ok": true, "username": req.username })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<LoginResp>> {
    let user = db::get_json::<UserRecord>(state.db.as_ref(), db::USERS, &req.username)?
        .ok_or_else(|| ApiError::unauthorized("用户名或密码错误"))?;
    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::unauthorized("用户名或密码错误"));
    }
    if user.disabled {
        return Err(ApiError::forbidden("账号已被管理员禁用"));
    }
    let is_admin = user.is_admin();
    let must_change_password = user.must_change_password;
    let token = random_token();
    db::put_json(
        state.db.as_ref(),
        db::TOKENS,
        &token,
        &new_token_record(&user.username),
    )?;
    Ok(Json(LoginResp {
        token,
        username: user.username,
        is_admin,
        must_change_password,
    }))
}

/// 有状态 token：登出即删除，服务端可随时吊销。
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    _user: AuthUser,
) -> ApiResult<Json<Value>> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .trim();
    if !token.is_empty() {
        db::remove_key(state.db.as_ref(), db::TOKENS, token)?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn me(user: AuthUser) -> ApiResult<Json<UserView>> {
    Ok(Json(UserView {
        username: user.0.username,
        role: user.0.role,
        disabled: user.0.disabled,
        created_at: user.0.created_at,
    }))
}

pub async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ChangePasswordReq>,
) -> ApiResult<Json<Value>> {
    validate_password(&req.new_password)?;
    let mut user = user.0;
    if !verify_password(&req.old_password, &user.password_hash) {
        return Err(ApiError::bad_request("原密码不正确"));
    }
    user.password_hash = hash_password(&req.new_password)?;
    user.must_change_password = false;
    db::put_json(state.db.as_ref(), db::USERS, &user.username, &user)?;
    Ok(Json(json!({ "ok": true })))
}

/// 公开信息：页面标题与是否开放注册。
pub async fn info(State(state): State<AppState>) -> Json<Value> {
    let config = state.config_snapshot();
    Json(json!({
        "name": config.server_name,
        "version": env!("CARGO_PKG_VERSION"),
        "allow_registration": config.allow_registration,
    }))
}

/// 注册用户列表，聊天界面展示用户名用。
pub async fn list_users(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<Vec<UserView>>> {
    let users: Vec<UserRecord> = db::all_json(state.db.as_ref(), db::USERS)?;
    let mut views: Vec<UserView> = users
        .into_iter()
        .map(|u| UserView {
            username: u.username,
            role: u.role,
            disabled: u.disabled,
            created_at: u.created_at,
        })
        .collect();
    views.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(Json(views))
}
