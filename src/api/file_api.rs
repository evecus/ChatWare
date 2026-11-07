use crate::auth::AuthUser;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::FileMeta;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

fn safe_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\r' | '\n' | '\0' | '"'))
        .take(120)
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "file.bin".to_string()
    } else {
        cleaned
    }
}

/// RFC 5987 文件名编码，同时防止响应头注入。
fn encode_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn header(map: &mut HeaderMap, name: axum::http::HeaderName, value: String) -> ApiResult<()> {
    let parsed = HeaderValue::from_str(&value).map_err(ApiError::internal)?;
    map.insert(name, parsed);
    Ok(())
}

/// 单请求流式上传：边收边写磁盘，超过上限立即删除临时文件。
pub async fn upload(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> ApiResult<Json<FileMeta>> {
    let max_size = state.config_snapshot().max_file_size_bytes();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(ApiError::bad_request)?
    {
        if field.name() != Some("file") {
            continue;
        }
        let name = safe_filename(field.file_name().unwrap_or("file.bin"));
        let mime = field.content_type().map(str::to_string).unwrap_or_else(|| {
            mime_guess::from_path(&name)
                .first_or_octet_stream()
                .to_string()
        });

        let id = Uuid::new_v4().to_string();
        let path = state.files_dir.join(&id);
        let mut file = tokio::fs::File::create(&path)
            .await
            .map_err(ApiError::internal)?;
        let mut size = 0u64;
        let mut field = field;
        while let Some(chunk) = field.chunk().await.map_err(ApiError::bad_request)? {
            size = size.saturating_add(chunk.len() as u64);
            if size > max_size {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err(ApiError::too_large(format!(
                    "文件超过上限 {} MB",
                    state.config_snapshot().max_file_size_mb
                )));
            }
            file.write_all(&chunk).await.map_err(ApiError::internal)?;
        }
        file.flush().await.map_err(ApiError::internal)?;
        drop(file);

        let meta = FileMeta {
            id,
            name,
            size,
            mime,
            uploader: user.0.username,
            created_at: crate::auth::now_ms(),
        };
        db::put_json(state.db.as_ref(), db::FILES, &meta.id, &meta)?;
        tracing::info!(file = %meta.name, size = meta.size, "文件上传完成");
        return Ok(Json(meta));
    }
    Err(ApiError::bad_request("缺少 file 字段"))
}

/// 任意登录用户都可下载其他人分享的文件。
pub async fn download(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let meta = db::get_json::<FileMeta>(state.db.as_ref(), db::FILES, &id)?
        .ok_or_else(|| ApiError::not_found("文件不存在"))?;
    let path = state.files_dir.join(&id);
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| ApiError::not_found("文件内容已丢失"))?;
    let body = Body::from_stream(ReaderStream::new(file));

    let mut headers = HeaderMap::new();
    header(&mut headers, CONTENT_TYPE, meta.mime.clone())?;
    header(
        &mut headers,
        CONTENT_DISPOSITION,
        format!(
            "attachment; filename=\"file\"; filename*=UTF-8''{}",
            encode_filename(&meta.name)
        ),
    )?;
    header(&mut headers, CONTENT_LENGTH, meta.size.to_string())?;
    Ok((headers, body).into_response())
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<Vec<FileMeta>>> {
    let mut files: Vec<FileMeta> = db::all_json(state.db.as_ref(), db::FILES)?;
    files.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(Json(files))
}
