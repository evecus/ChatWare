use crate::config::Config;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::ConfigUpdateReq;
use crate::ratelimit::{RateLimiter, SharedRateLimiter};
use redb::Database;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Arc<RwLock<Config>>,
    /// 单一公共大厅的广播通道
    pub hub: broadcast::Sender<String>,
    pub files_dir: Arc<PathBuf>,
    /// 登录 / 注册 / WS 鉴权防爆破限流（内存态，重启清零）
    pub limiter: SharedRateLimiter,
    pub started_at: i64,
}

impl AppState {
    pub fn new(db: Database, mut config: Config) -> Self {
        let settings = db::load_settings(&db).unwrap_or_else(|e| {
            eprintln!("[chatware] 运行期配置加载失败: {}", e.message());
            std::process::exit(1);
        });
        config
            .apply_runtime_overrides(&settings)
            .unwrap_or_else(|e| {
                eprintln!("[chatware] 运行期配置加载失败: {e}");
                std::process::exit(1);
            });
        let files_dir = config.data_dir.join("files");
        std::fs::create_dir_all(&files_dir).expect("无法创建文件存储目录");
        let (hub, _) = broadcast::channel(256);
        Self {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(config)),
            hub,
            files_dir: Arc::new(files_dir),
            limiter: Arc::new(RateLimiter::default()),
            started_at: crate::auth::now_ms(),
        }
    }

    pub fn config_snapshot(&self) -> Config {
        self.config.read().expect("配置读写锁已失效").clone()
    }

    /// fail-fast：校验不通过直接拒绝，不静默使用旧值。
    /// 校验规则与启动加载共用 Config::validate，避免多处规则不一致。
    pub fn apply_config_update(&self, req: ConfigUpdateReq) -> ApiResult<Config> {
        let mut current = self.config_snapshot();
        if let Some(v) = req.allow_registration {
            current.allow_registration = v;
        }
        if let Some(v) = req.max_file_size_mb {
            current.max_file_size_mb = v;
        }
        if let Some(v) = req.max_message_length {
            current.max_message_length = v;
        }
        if let Some(v) = req.server_name {
            current.server_name = v;
        }
        if let Some(v) = req.public_url {
            current.public_url = v;
        }
        current
            .normalize_and_validate()
            .map_err(ApiError::bad_request)?;
        db::save_settings(self.db.as_ref(), &current.runtime_settings())?;
        *self.config.write().expect("配置读写锁已失效") = current.clone();
        Ok(current)
    }
}
