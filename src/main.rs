mod api;
mod auth;
mod config;
mod db;
mod error;
mod models;
mod state;
mod web;

use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post};
use axum::Router;
use clap::Parser;
use config::{Cli, Config};
use state::AppState;

/// 传输层硬上限，与运行期可配的 max_file_size_mb 解耦：
/// 管理面板把上限调大后无需重启即可生效，真实限制在 upload handler 内按配置检查。
const HARD_BODY_LIMIT: usize = 1100 * 1024 * 1024;

fn router(state: AppState) -> Router {
    Router::new()
        // 页面与静态资源（全部由 src/web/*.rs 生成）
        .route("/", get(web::index))
        .route("/user", get(web::user_page))
        .route("/admin", get(web::admin_page))
        .route("/assets/style.css", get(web::css))
        .route("/assets/user.js", get(web::user_js))
        .route("/assets/admin.js", get(web::admin_js))
        // 鉴权与账号
        .route("/api/info", get(api::auth_api::info))
        .route("/api/register", post(api::auth_api::register))
        .route("/api/login", post(api::auth_api::login))
        .route("/api/logout", post(api::auth_api::logout))
        .route("/api/me", get(api::auth_api::me))
        .route("/api/change-password", post(api::auth_api::change_password))
        .route("/api/users", get(api::auth_api::list_users))
        // 聊天
        .route("/api/messages", get(api::chat_api::history))
        .route("/api/ws", get(api::chat_api::ws_handler))
        // 文件
        .route("/api/files", post(api::file_api::upload))
        .route("/api/files", get(api::file_api::list))
        .route("/api/files/:id", get(api::file_api::download))
        // 管理接口
        .route("/api/admin/summary", get(api::admin_api::summary))
        .route(
            "/api/admin/invites",
            get(api::admin_api::list_invites).post(api::admin_api::create_invite),
        )
        .route(
            "/api/admin/invites/:code",
            delete(api::admin_api::delete_invite),
        )
        .route("/api/admin/users", get(api::admin_api::list_users))
        .route(
            "/api/admin/users/:username/disable",
            post(api::admin_api::disable_user),
        )
        .route(
            "/api/admin/users/:username/enable",
            post(api::admin_api::enable_user),
        )
        .route(
            "/api/admin/users/:username",
            delete(api::admin_api::delete_user),
        )
        .route(
            "/api/admin/config",
            get(api::admin_api::get_config).post(api::admin_api::update_config),
        )
        .route(
            "/api/admin/connection-info",
            get(api::admin_api::connection_info),
        )
        .layer(DefaultBodyLimit::max(HARD_BODY_LIMIT))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chatware=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let config = Config::load(&cli).unwrap_or_else(|e| {
        eprintln!("[chatware] 配置加载失败: {e}");
        std::process::exit(1);
    });

    let data_dir = config.data_dir.clone();
    std::fs::create_dir_all(&data_dir).unwrap_or_else(|e| {
        eprintln!("[chatware] 无法创建数据目录 {}: {e}", data_dir.display());
        std::process::exit(1);
    });
    let db_path = data_dir.join("chatware.redb");
    let database = db::open_db(&db_path).unwrap_or_else(|e| {
        eprintln!("[chatware] 打开数据库失败: {}", e.message());
        std::process::exit(1);
    });
    db::init_tables(&database).unwrap_or_else(|e| {
        eprintln!("[chatware] 初始化数据表失败: {}", e.message());
        std::process::exit(1);
    });
    auth::ensure_admin(&database, &data_dir).unwrap_or_else(|e| {
        eprintln!("[chatware] 初始化管理员失败: {}", e.message());
        std::process::exit(1);
    });

    let bind = config.bind.clone();
    let state = AppState::new(database, config);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| {
            eprintln!("[chatware] 监听 {bind} 失败: {e}");
            std::process::exit(1);
        });

    tracing::info!(
        "Chatware v{} 已启动: http://{bind}",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("管理面板 /admin · 用户面板 /user");
    tracing::info!("本程序只监听 HTTP，TLS 域名证书请由反向代理提供");

    axum::serve(listener, router(state))
        .await
        .unwrap_or_else(|e| {
            eprintln!("[chatware] 服务运行错误: {e}");
            std::process::exit(1);
        });
}
