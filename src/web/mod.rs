pub mod admin;
pub mod assets;
pub mod user;

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse};

/// 根路径是对外连接地址本身（客户端把 https://域名 当作服务端地址，直接请求 /api/*）。
/// 这里只返回一个空的成功响应，不渲染任何页面、也不重定向到 /user。
pub async fn index() -> impl IntoResponse {
    (StatusCode::OK, "")
}

fn asset(content: &'static str, mime: &'static str) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(mime));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    (headers, content)
}

pub async fn css() -> impl IntoResponse {
    asset(assets::CSS, "text/css; charset=utf-8")
}

pub async fn user_js() -> impl IntoResponse {
    asset(user::JS, "application/javascript; charset=utf-8")
}

pub async fn admin_js() -> impl IntoResponse {
    asset(admin::JS, "application/javascript; charset=utf-8")
}

pub async fn user_page() -> Html<&'static str> {
    Html(user::PAGE)
}

pub async fn admin_page() -> Html<&'static str> {
    Html(admin::PAGE)
}
