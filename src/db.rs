use crate::config::RuntimeSettings;
use crate::error::{ApiError, ApiResult};
use crate::models::{Message, TokenRecord};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::Path;

/// 业务表统一使用 &str 键 + JSON 字节值，避免引入额外索引机制。
pub const USERS: TableDefinition<&str, &[u8]> = TableDefinition::new("users");
pub const TOKENS: TableDefinition<&str, &[u8]> = TableDefinition::new("tokens");
pub const INVITES: TableDefinition<&str, &[u8]> = TableDefinition::new("invites");
pub const FILES: TableDefinition<&str, &[u8]> = TableDefinition::new("files");
/// 键格式：{20 位毫秒时间戳}:{uuid}，字典序即时间序
pub const MESSAGES: TableDefinition<&str, &[u8]> = TableDefinition::new("messages");
/// 运行期配置：固定 key + 纯 UTF-8 文本值（"true" / "256" / 原文），不经过 JSON 序列化。
/// 值类型与业务表一致，仅编码方式不同。
pub const SETTINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");
/// 服务器管理员：固定 key "password_hash" + argon2 哈希纯文本。
/// 管理员不属于用户体系，绝不写入 USERS 表，因此 "admin" 只是普通用户名。
pub const ADMIN: TableDefinition<&str, &[u8]> = TableDefinition::new("admin");
/// 管理员会话：key = token，value = 过期毫秒时间戳纯文本。与用户 TOKENS 表完全隔离。
pub const ADMIN_TOKENS: TableDefinition<&str, &[u8]> = TableDefinition::new("admin_tokens");

pub const ADMIN_PASSWORD_KEY: &str = "password_hash";

/// 大于任何合法消息键的上界，用于「拉取最新一页」
const MAX_KEY: &str = "\u{10ffff}";

pub fn open_db(path: &Path) -> ApiResult<Database> {
    Database::create(path).map_err(ApiError::internal)
}

pub fn init_tables(db: &Database) -> ApiResult<()> {
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        // open_table 会建表；借用必须在 commit 前结束
        let _ = txn.open_table(USERS).map_err(ApiError::internal)?;
        let _ = txn.open_table(TOKENS).map_err(ApiError::internal)?;
        let _ = txn.open_table(INVITES).map_err(ApiError::internal)?;
        let _ = txn.open_table(FILES).map_err(ApiError::internal)?;
        let _ = txn.open_table(MESSAGES).map_err(ApiError::internal)?;
        let _ = txn.open_table(SETTINGS).map_err(ApiError::internal)?;
        let _ = txn.open_table(ADMIN).map_err(ApiError::internal)?;
        let _ = txn.open_table(ADMIN_TOKENS).map_err(ApiError::internal)?;
    }
    txn.commit().map_err(ApiError::internal)
}

/// 读取运行期配置：把 settings 表的 (key, value) 交给 RuntimeSettings 解码。
/// 非法 UTF-8、未知 key、非法值都会报错（fail-fast），不静默忽略。
pub fn load_settings(db: &Database) -> ApiResult<RuntimeSettings> {
    let txn = db.begin_read().map_err(ApiError::internal)?;
    let table = txn.open_table(SETTINGS).map_err(ApiError::internal)?;
    let mut pairs = Vec::new();
    for entry in table.iter().map_err(ApiError::internal)? {
        let (key, value) = entry.map_err(ApiError::internal)?;
        let text = std::str::from_utf8(value.value())
            .map_err(|e| ApiError::internal(format!("settings 表值不是 UTF-8: {e}")))?;
        pairs.push((key.value().to_string(), text.to_string()));
    }
    RuntimeSettings::from_pairs(&pairs).map_err(ApiError::internal)
}

/// 整体替换运行期配置：同一个写事务内清空旧值再写入，保证不会出现半套配置。
pub fn save_settings(db: &Database, settings: &RuntimeSettings) -> ApiResult<()> {
    let pairs = settings.to_pairs();
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        let mut table = txn.open_table(SETTINGS).map_err(ApiError::internal)?;
        let keys: Vec<String> = table
            .iter()
            .map_err(ApiError::internal)?
            .map(|entry| entry.map(|(key, _)| key.value().to_string()))
            .collect::<Result<_, _>>()
            .map_err(ApiError::internal)?;
        for key in &keys {
            table.remove(key.as_str()).map_err(ApiError::internal)?;
        }
        for (key, value) in &pairs {
            table
                .insert(*key, value.as_bytes())
                .map_err(ApiError::internal)?;
        }
    }
    txn.commit().map_err(ApiError::internal)
}

pub fn get_json<T: DeserializeOwned>(
    db: &Database,
    def: TableDefinition<&str, &[u8]>,
    key: &str,
) -> ApiResult<Option<T>> {
    let txn = db.begin_read().map_err(ApiError::internal)?;
    let table = txn.open_table(def).map_err(ApiError::internal)?;
    let Some(guard) = table.get(key).map_err(ApiError::internal)? else {
        return Ok(None);
    };
    let value = serde_json::from_slice(guard.value()).map_err(ApiError::internal)?;
    Ok(Some(value))
}

pub fn put_json<T: Serialize>(
    db: &Database,
    def: TableDefinition<&str, &[u8]>,
    key: &str,
    value: &T,
) -> ApiResult<()> {
    let bytes = serde_json::to_vec(value).map_err(ApiError::internal)?;
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        let mut table = txn.open_table(def).map_err(ApiError::internal)?;
        table
            .insert(key, bytes.as_slice())
            .map_err(ApiError::internal)?;
    }
    txn.commit().map_err(ApiError::internal)
}

pub fn remove_key(db: &Database, def: TableDefinition<&str, &[u8]>, key: &str) -> ApiResult<()> {
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        let mut table = txn.open_table(def).map_err(ApiError::internal)?;
        table.remove(key).map_err(ApiError::internal)?;
    }
    txn.commit().map_err(ApiError::internal)
}

/// 读取纯 UTF-8 文本值（管理员密码哈希 / 管理员 token 过期时间都用它，不经过 JSON）。
pub fn get_text(
    db: &Database,
    def: TableDefinition<&str, &[u8]>,
    key: &str,
) -> ApiResult<Option<String>> {
    let txn = db.begin_read().map_err(ApiError::internal)?;
    let table = txn.open_table(def).map_err(ApiError::internal)?;
    let Some(guard) = table.get(key).map_err(ApiError::internal)? else {
        return Ok(None);
    };
    let text = std::str::from_utf8(guard.value())
        .map_err(|e| ApiError::internal(format!("表值不是 UTF-8: {e}")))?;
    Ok(Some(text.to_string()))
}

/// 写入纯 UTF-8 文本值。
pub fn put_text(
    db: &Database,
    def: TableDefinition<&str, &[u8]>,
    key: &str,
    value: &str,
) -> ApiResult<()> {
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        let mut table = txn.open_table(def).map_err(ApiError::internal)?;
        table
            .insert(key, value.as_bytes())
            .map_err(ApiError::internal)?;
    }
    txn.commit().map_err(ApiError::internal)
}

/// 清空整张表：改密时一次性吊销全部管理员会话。
pub fn clear_table(db: &Database, def: TableDefinition<&str, &[u8]>) -> ApiResult<()> {
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        let mut table = txn.open_table(def).map_err(ApiError::internal)?;
        let keys: Vec<String> = table
            .iter()
            .map_err(ApiError::internal)?
            .map(|entry| entry.map(|(key, _)| key.value().to_string()))
            .collect::<Result<_, _>>()
            .map_err(ApiError::internal)?;
        for key in &keys {
            table.remove(key.as_str()).map_err(ApiError::internal)?;
        }
    }
    txn.commit().map_err(ApiError::internal)
}

pub fn all_json<T: DeserializeOwned>(
    db: &Database,
    def: TableDefinition<&str, &[u8]>,
) -> ApiResult<Vec<T>> {
    let txn = db.begin_read().map_err(ApiError::internal)?;
    let table = txn.open_table(def).map_err(ApiError::internal)?;
    let mut out = Vec::new();
    for entry in table.iter().map_err(ApiError::internal)? {
        let (_, value) = entry.map_err(ApiError::internal)?;
        out.push(serde_json::from_slice(value.value()).map_err(ApiError::internal)?);
    }
    Ok(out)
}

pub fn count(db: &Database, def: TableDefinition<&str, &[u8]>) -> ApiResult<u64> {
    let txn = db.begin_read().map_err(ApiError::internal)?;
    let table = txn.open_table(def).map_err(ApiError::internal)?;
    table.len().map_err(ApiError::internal)
}

/// 删除用户 / 禁用账号时，一并吊销该用户名下的所有 token。
pub fn revoke_user_tokens(db: &Database, username: &str) -> ApiResult<()> {
    let mut keys = Vec::new();
    {
        let txn = db.begin_read().map_err(ApiError::internal)?;
        let table = txn.open_table(TOKENS).map_err(ApiError::internal)?;
        for entry in table.iter().map_err(ApiError::internal)? {
            let (key, value) = entry.map_err(ApiError::internal)?;
            let record: TokenRecord =
                serde_json::from_slice(value.value()).map_err(ApiError::internal)?;
            if record.username == username {
                keys.push(key.value().to_string());
            }
        }
    }
    if keys.is_empty() {
        return Ok(());
    }
    let txn = db.begin_write().map_err(ApiError::internal)?;
    {
        let mut table = txn.open_table(TOKENS).map_err(ApiError::internal)?;
        for key in &keys {
            table.remove(key.as_str()).map_err(ApiError::internal)?;
        }
    }
    txn.commit().map_err(ApiError::internal)
}

pub fn message_key(ts: i64, id: &str) -> String {
    format!("{ts:020}:{id}")
}

/// 取 before 之前的 limit 条，按时间升序返回；has_more 表示是否还有更早的消息。
/// 只保留最后 limit+1 个键，避免把整张表反序列化进内存。
pub fn list_messages(
    db: &Database,
    before: Option<&str>,
    limit: usize,
) -> ApiResult<(Vec<Message>, bool)> {
    let limit = limit.clamp(1, 100);
    let end = before.filter(|s| !s.is_empty()).unwrap_or(MAX_KEY);
    let txn = db.begin_read().map_err(ApiError::internal)?;
    let table = txn.open_table(MESSAGES).map_err(ApiError::internal)?;

    let mut tail: VecDeque<String> = VecDeque::with_capacity(limit + 1);
    for entry in table.range(..end).map_err(ApiError::internal)? {
        let (key, _) = entry.map_err(ApiError::internal)?;
        if tail.len() > limit {
            tail.pop_front();
        }
        tail.push_back(key.value().to_string());
    }

    let has_more = tail.len() > limit;
    if has_more {
        tail.pop_front();
    }
    let mut messages = Vec::with_capacity(tail.len());
    for key in tail {
        let Some(guard) = table.get(key.as_str()).map_err(ApiError::internal)? else {
            continue;
        };
        messages.push(serde_json::from_slice(guard.value()).map_err(ApiError::internal)?);
    }
    Ok((messages, has_more))
}
