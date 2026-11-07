use crate::error::{ApiError, ApiResult};
use crate::models::{Message, TokenRecord};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::Path;

/// 所有表统一使用 &str 键 + JSON 字节值，避免引入额外索引机制。
pub const USERS: TableDefinition<&str, &[u8]> = TableDefinition::new("users");
pub const TOKENS: TableDefinition<&str, &[u8]> = TableDefinition::new("tokens");
pub const INVITES: TableDefinition<&str, &[u8]> = TableDefinition::new("invites");
pub const FILES: TableDefinition<&str, &[u8]> = TableDefinition::new("files");
/// 键格式：{20 位毫秒时间戳}:{uuid}，字典序即时间序
pub const MESSAGES: TableDefinition<&str, &[u8]> = TableDefinition::new("messages");

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
