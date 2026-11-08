use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
    /// 429 时建议客户端等待的秒数，写入 Retry-After 响应头
    retry_after_secs: Option<i64>,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl std::fmt::Display) -> Self {
        Self {
            status,
            message: message.to_string(),
            retry_after_secs: None,
        }
    }

    pub fn bad_request(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn conflict(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    pub fn too_large(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, message)
    }

    /// 防爆破触发：429 + Retry-After（秒）。
    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn too_many_requests(message: impl std::fmt::Display, retry_after_secs: i64) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: message.to_string(),
            retry_after_secs: Some(retry_after_secs.max(1)),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn retry_after_secs(&self) -> i64 {
        self.retry_after_secs.unwrap_or(0)
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(json!({ "error": self.message }))).into_response();
        let secs = self.retry_after_secs();
        if secs > 0 {
            if let Ok(value) = secs.to_string().parse() {
                response.headers_mut().insert("retry-after", value);
            }
        }
        response
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
