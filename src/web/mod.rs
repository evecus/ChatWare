pub mod admin;
pub mod assets;
pub mod user;

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{Html, IntoResponse, Redirect};

pub async fn index() -> Redirect {
    Redirect::temporary("/user")
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
