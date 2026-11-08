use crate::auth::{
    hash_password, new_token_record, now_ms, random_token, validate_password, validate_username,
    verify_password, AuthUser,
};
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::{
    AccountUpdateReq, AccountUpdateResp, InviteRecord, LoginReq, LoginResp, RegisterReq,
    UserRecord, UserView,
};
use crate::ratelimit::client_keys;
use crate::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use rand::Rng;
use redb::ReadableTable;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;

/// 登录失败：计入防爆破 + 轻微随机延迟，配合 argon2 拉平爆破成本。
async fn login_failed(state: &AppState, primary: &str, peer: &str) -> ApiError {
    state.limiter.fail("user-login", primary, peer);
    tracing::warn!(client = %primary, peer = %peer, "用户登录失败（计入防爆破）");
    // 先取值再 await：ThreadRng 不是 Send，不能活过等待点
    let delay = rand::thread_rng().gen_range(100..400u64);
    tokio::time::sleep(Duration::from_millis(delay)).await;
    ApiError::unauthorized("用户名或密码错误")
}

/// 注册邀请码失败：计入防爆破，防止邀请码被穷举。
async fn invite_failed(state: &AppState, primary: &str, peer: &str, msg: &str) -> ApiError {
    state.limiter.fail("register", primary, peer);
    ApiError::bad_request(msg)
}

/// 注册：邀请码核销与账号创建放在同一个写事务里，避免并发超发。
pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<Value>> {
    let (primary, peer_ip) = client_keys(&headers, peer);
    state.limiter.verify("register", &primary, &peer_ip)?;
    if !state.config_snapshot().allow_registration {
        return Err(ApiError::forbidden("管理员已关闭注册"));
    }
    validate_username(&req.username)?;
    validate_password(&req.password)?;
    let code = req.invite_code.trim().to_uppercase();
    if code.is_empty() {
        return Err(ApiError::bad_request("请输入邀请码"));
    }
    // 邀请码预检（只读），失败计入防爆破；正式核销仍在下方写事务内复核
    let invite: InviteRecord =
        match db::get_json::<InviteRecord>(state.db.as_ref(), db::INVITES, &code)? {
            Some(i) => i,
            None => return Err(invite_failed(&state, &primary, &peer_ip, "邀请码无效").await),
        };
    if invite.used >= invite.max_uses {
        return Err(invite_failed(&state, &primary, &peer_ip, "邀请码使用次数已用尽").await);
    }
    if invite.expires_at != 0 && invite.expires_at <= now_ms() {
        return Err(invite_failed(&state, &primary, &peer_ip, "邀请码已过期").await);
    }
    if db::get_json::<UserRecord>(state.db.as_ref(), db::USERS, &req.username)?.is_some() {
        return Err(ApiError::conflict("用户名已存在"));
    }

    let password_hash = hash_password(&req.password)?;
    let record = UserRecord {
        username: req.username.clone(),
        password_hash,
        disabled: false,
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
    state.limiter.succeed("register", &primary, &peer_ip);

    tracing::info!(username = %req.username, "新用户注册");
    Ok(Json(json!({ "ok": true, "username": req.username })))
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<LoginResp>> {
    let (primary, peer_ip) = client_keys(&headers, peer);
    state.limiter.verify("user-login", &primary, &peer_ip)?;
    let user = match db::get_json::<UserRecord>(state.db.as_ref(), db::USERS, &req.username)? {
        Some(u) => u,
        None => return Err(login_failed(&state, &primary, &peer_ip).await),
    };
    if !verify_password(&req.password, &user.password_hash) {
        return Err(login_failed(&state, &primary, &peer_ip).await);
    }
    state.limiter.succeed("user-login", &primary, &peer_ip);
    if user.disabled {
        return Err(ApiError::forbidden("账号已被管理员禁用"));
    }
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
        disabled: user.0.disabled,
        created_at: user.0.created_at,
    }))
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
            disabled: u.disabled,
            created_at: u.created_at,
        })
        .collect();
    views.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(Json(views))
}

/// 用户自助修改账号：验证当前密码后可改用户名 / 密码。
/// 改名会在同一写事务里同步重写历史消息与文件署名，并吊销该用户全部会话。
pub async fn update_account(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    user: AuthUser,
    Json(req): Json<AccountUpdateReq>,
) -> ApiResult<Json<AccountUpdateResp>> {
    let (primary, peer_ip) = client_keys(&headers, peer);
    state.limiter.verify("account", &primary, &peer_ip)?;
    let old_username = user.0.username.clone();
    if !verify_password(&req.current_password, &user.0.password_hash) {
        state.limiter.fail("account", &primary, &peer_ip);
        tracing::warn!(client = %primary, user = %old_username, "修改账号时当前密码验证失败（计入防爆破）");
        return Err(ApiError::bad_request("当前密码不正确"));
    }
    state.limiter.succeed("account", &primary, &peer_ip);
    let new_username = req.username.trim();
    validate_username(new_username)?;
    let renamed = new_username != old_username.as_str();
    let change_password = !req.new_password.is_empty();
    if change_password {
        validate_password(&req.new_password)?;
    }
    if !renamed && !change_password {
        return Ok(Json(AccountUpdateResp {
            ok: true,
            username: old_username,
            relogin: false,
        }));
    }
    let new_hash = if change_password {
        Some(hash_password(&req.new_password)?)
    } else {
        None
    };
    db::update_user_account(
        state.db.as_ref(),
        &old_username,
        new_username,
        new_hash.as_deref(),
    )?;
    tracing::info!(
        old = %old_username,
        new = %new_username,
        renamed,
        change_password,
        "用户修改账号信息"
    );
    Ok(Json(AccountUpdateResp {
        ok: true,
        username: new_username.to_string(),
        relogin: true,
    }))
}
