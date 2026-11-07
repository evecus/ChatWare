use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub username: String,
    pub password_hash: String,
    pub disabled: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRecord {
    pub username: String,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteRecord {
    pub code: String,
    pub created_by: String,
    pub max_uses: u32,
    pub used: u32,
    /// 0 表示永不过期
    pub expires_at: i64,
    pub note: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub uploader: String,
    pub created_at: i64,
}

/// 一条聊天消息。text 类型用 text 字段，file 类型用 file 字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    /// 数据库有序键，客户端翻页时作为 before 参数回传
    pub key: String,
    pub ts: i64,
    pub username: String,
    /// "text" | "file"
    pub kind: String,
    pub text: String,
    pub file: Option<FileMeta>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterReq {
    pub username: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

/// 管理面板登录：只需要密码。管理员是服务器级身份，不属于用户体系、没有用户名。
#[derive(Debug, Deserialize)]
pub struct AdminLoginReq {
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteReq {
    #[serde(default)]
    pub max_uses: Option<u32>,
    #[serde(default)]
    pub expires_hours: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientMessage {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub file_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdateReq {
    #[serde(default)]
    pub allow_registration: Option<bool>,
    #[serde(default)]
    pub max_file_size_mb: Option<u64>,
    #[serde(default)]
    pub max_message_length: Option<usize>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub public_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResp {
    pub token: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct UserView {
    pub username: String,
    pub disabled: bool,
    pub created_at: i64,
}
