use crate::auth::{now_ms, random_code, AdminUser};
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::{ConfigUpdateReq, CreateInviteReq, InviteRecord, UserRecord, UserView};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

pub async fn summary(State(state): State<AppState>, _admin: AdminUser) -> ApiResult<Json<Value>> {
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
    _admin: AdminUser,
) -> ApiResult<Json<Vec<InviteRecord>>> {
    let mut invites: Vec<InviteRecord> = db::all_json(state.db.as_ref(), db::INVITES)?;
    invites.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    Ok(Json(invites))
}

pub async fn create_invite(
    State(state): State<AppState>,
    admin: AdminUser,
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
        created_by: admin.0.username,
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
    _admin: AdminUser,
    Path(code): Path<String>,
) -> ApiResult<Json<Value>> {
    db::remove_key(state.db.as_ref(), db::INVITES, &code)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
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
    admin: AdminUser,
    Path(username): Path<String>,
) -> ApiResult<Json<Value>> {
    if username == admin.0.username {
        return Err(ApiError::bad_request("不能禁用当前管理员账号"));
    }
    set_disabled(&state, &username, true)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn enable_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(username): Path<String>,
) -> ApiResult<Json<Value>> {
    set_disabled(&state, &username, false)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(username): Path<String>,
) -> ApiResult<Json<Value>> {
    if username == admin.0.username {
        return Err(ApiError::bad_request("不能删除当前管理员账号"));
    }
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
    _admin: AdminUser,
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
    _admin: AdminUser,
    Json(req): Json<ConfigUpdateReq>,
) -> ApiResult<Json<Value>> {
    let config = state.apply_config_update(req)?;
    Ok(Json(json!({
        "ok": true,
        "allow_registration": config.allow_registration,
        "max_file_size_mb": config.max_file_size_mb,
    })))
}

/// TLS 由反向代理提供，这里给出客户端「手动填指纹」所需的提示。
pub async fn connection_info(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> ApiResult<Json<Value>> {
    let config = state.config_snapshot();
    let raw = if config.public_url.is_empty() {
        format!("http://{}", config.bind)
    } else {
        config.public_url.clone()
    };
    let host_port = raw
        .split("://")
        .nth(1)
        .unwrap_or(raw.as_str())
        .split('/')
        .next()
        .unwrap_or("HOST")
        .to_string();
    let host = host_port.split(':').next().unwrap_or("HOST").to_string();
    Ok(Json(json!({
        "public_url": raw,
        "tls": "由反向代理提供域名证书；本程序只监听 HTTP",
        "fingerprint_command": format!(
            "openssl s_client -connect {host_port} -servername {host} </dev/null 2>/dev/null | openssl x509 -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256 -binary | openssl base64"
        ),
        "note": "在安卓客户端「证书指纹」中填入上面命令的输出；若使用公网可信证书，可保持为空。",
    })))
}
