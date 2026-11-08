use crate::auth::{admin_login, change_admin_password, now_ms, random_code, AdminAuth};
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::{
    AdminLoginReq, AdminPasswordReq, ConfigUpdateReq, CreateInviteReq, InviteRecord, UserRecord,
    UserView,
};
use crate::ratelimit::client_keys;
use crate::state::AppState;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use rand::Rng;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;

/// 管理面板登录：只提交密码，通过后签发管理员会话 token。
/// 失败计入防爆破并带随机延迟；这是最高价值目标，阈值与用户登录相同但单独立表。
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<AdminLoginReq>,
) -> ApiResult<Json<Value>> {
    let (primary, peer_ip) = client_keys(&headers, peer);
    state.limiter.verify("admin-login", &primary, &peer_ip)?;
    match admin_login(&state, &req.password) {
        Ok(token) => {
            state.limiter.succeed("admin-login", &primary, &peer_ip);
            Ok(Json(json!({ "token": token })))
        }
        Err(e) if e.status() == StatusCode::UNAUTHORIZED => {
            state.limiter.fail("admin-login", &primary, &peer_ip);
            tracing::warn!(client = %primary, peer = %peer_ip, "管理面板登录失败（计入防爆破）");
            // 先取值再 await：ThreadRng 不是 Send，不能活过等待点
            let delay = rand::thread_rng().gen_range(100..400u64);
            tokio::time::sleep(Duration::from_millis(delay)).await;
            Err(e)
        }
        Err(e) => Err(e),
    }
}

/// 登出即删除该管理员会话 token，服务端立即失效。
pub async fn logout(
    State(state): State<AppState>,
    _admin: AdminAuth,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .trim();
    if !token.is_empty() {
        db::remove_key(state.db.as_ref(), db::ADMIN_TOKENS, token)?;
    }
    Ok(Json(json!({ "ok": true })))
}

/// 会话有效性校验：前端带 token 探测，有效返回 200。
pub async fn me(_admin: AdminAuth) -> Json<Value> {
    Json(json!({ "ok": true }))
}

/// 在线修改管理密码：验证当前密码，成功后吊销全部管理员会话（含当前）。
/// 当前密码验证失败同样计入防爆破，避免拿到会话后离线穷举管理密码。
pub async fn change_password(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    _admin: AdminAuth,
    Json(req): Json<AdminPasswordReq>,
) -> ApiResult<Json<Value>> {
    let (primary, peer_ip) = client_keys(&headers, peer);
    state.limiter.verify("admin-password", &primary, &peer_ip)?;
    match change_admin_password(state.db.as_ref(), &req.current_password, &req.new_password) {
        Ok(()) => {
            state.limiter.succeed("admin-password", &primary, &peer_ip);
            tracing::info!("管理密码已通过面板修改，全部管理员会话已吊销");
            Ok(Json(json!({ "ok": true })))
        }
        Err(e) if e.status() == StatusCode::BAD_REQUEST => {
            state.limiter.fail("admin-password", &primary, &peer_ip);
            Err(e)
        }
        Err(e) => Err(e),
    }
}

pub async fn summary(State(state): State<AppState>, _admin: AdminAuth) -> ApiResult<Json<Value>> {
    let users = db::count(state.db.as_ref(), db::USERS)?;
    let messages = db::count(state.db.as_ref(), db::MESSAGES)?;
    let files: Vec<crate::models::FileMeta> = db::all_json(state.db.as_ref(), db::FILES)?;
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let invites = db::count(state.db.as_ref(), db::INVITES)?;
    Ok(Json(json!({
        "users": users,
        "messages": messages,
        "files": files.len(),
        "total_size": total_size,
        "invites": invites,
        "started_at": state.started_at,
    })))
}

pub async fn list_invites(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<InviteRecord>>> {
    let mut invites: Vec<InviteRecord> = db::all_json(state.db.as_ref(), db::INVITES)?;
    invites.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    Ok(Json(invites))
}

pub async fn create_invite(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(req): Json<CreateInviteReq>,
) -> ApiResult<Json<InviteRecord>> {
    let max_uses = req.max_uses.unwrap_or(1).clamp(1, 1000);
    let hours = req.expires_hours.unwrap_or(0).clamp(0, 24 * 365);
    let code = loop {
        let candidate = random_code();
        if db::get_json::<InviteRecord>(state.db.as_ref(), db::INVITES, &candidate)?.is_none() {
            break candidate;
        }
    };
    let now = now_ms();
    let record = InviteRecord {
        code,
        // 管理员是服务器级身份、没有用户名；这里用固定标识表示"服务器管理员"。
        created_by: "server-admin".to_string(),
        max_uses,
        used: 0,
        expires_at: if hours > 0 {
            now.saturating_add(hours.saturating_mul(3600 * 1000))
        } else {
            0
        },
        note: req.note.unwrap_or_default(),
        created_at: now,
    };
    db::put_json(state.db.as_ref(), db::INVITES, &record.code, &record)?;
    tracing::info!(code = %record.code, "生成邀请码");
    Ok(Json(record))
}

pub async fn delete_invite(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Path(code): Path<String>,
) -> ApiResult<Json<Value>> {
    db::remove_key(state.db.as_ref(), db::INVITES, &code)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<UserView>>> {
    let users: Vec<UserRecord> = db::all_json(state.db.as_ref(), db::USERS)?;
    let mut views: Vec<UserView> = users
        .into_iter()
        .map(|u| UserView {
            username: u.username,
            disabled: u.disabled,
            created_at: u.created_at,
        })
        .collect();
    views.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(Json(views))
}

/// 禁用只置标志，token 保留：由 user_by_token 实时检查 disabled 拦截，
/// 启用后原有 token 立即恢复，不额外维护吊销列表。
fn set_disabled(state: &AppState, username: &str, disabled: bool) -> ApiResult<()> {
    let mut user = db::get_json::<UserRecord>(state.db.as_ref(), db::USERS, username)?
        .ok_or_else(|| ApiError::not_found("用户不存在"))?;
    user.disabled = disabled;
    db::put_json(state.db.as_ref(), db::USERS, username, &user)?;
    Ok(())
}

pub async fn disable_user(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Path(username): Path<String>,
) -> ApiResult<Json<Value>> {
    set_disabled(&state, &username, true)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn enable_user(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Path(username): Path<String>,
) -> ApiResult<Json<Value>> {
    set_disabled(&state, &username, false)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Path(username): Path<String>,
) -> ApiResult<Json<Value>> {
    if db::get_json::<UserRecord>(state.db.as_ref(), db::USERS, &username)?.is_none() {
        return Err(ApiError::not_found("用户不存在"));
    }
    db::revoke_user_tokens(state.db.as_ref(), &username)?;
    db::remove_key(state.db.as_ref(), db::USERS, &username)?;
    // 历史消息与已上传文件保留，避免其他人的聊天记录出现空洞
    Ok(Json(json!({ "ok": true })))
}

pub async fn get_config(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Value>> {
    let config = state.config_snapshot();
    Ok(Json(json!({
        "allow_registration": config.allow_registration,
        "max_file_size_mb": config.max_file_size_mb,
        "max_message_length": config.max_message_length,
        "bind": config.bind,
        "server_name": config.server_name,
        "public_url": config.public_url,
        "data_dir": config.data_dir.to_string_lossy(),
    })))
}

pub async fn update_config(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(req): Json<ConfigUpdateReq>,
) -> ApiResult<Json<Value>> {
    let config = state.apply_config_update(req)?;
    Ok(Json(json!({
        "ok": true,
        "allow_registration": config.allow_registration,
        "max_file_size_mb": config.max_file_size_mb,
        "max_message_length": config.max_message_length,
        "server_name": config.server_name,
        "public_url": config.public_url,
    })))
}

/// 反向代理提供真实域名证书，客户端只需对外地址，无需任何证书配置。
pub async fn connection_info(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Value>> {
    let config = state.config_snapshot();
    let public_url = if config.public_url.is_empty() {
        format!("http://{}", config.bind)
    } else {
        config.public_url
    };
    Ok(Json(json!({ "public_url": public_url })))
}
