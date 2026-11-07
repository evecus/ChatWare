use crate::config::{save_runtime_settings, Config, RuntimeSettings};
use crate::error::{ApiError, ApiResult};
use crate::models::ConfigUpdateReq;
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
    pub data_dir: Arc<PathBuf>,
    pub files_dir: Arc<PathBuf>,
    pub started_at: i64,
}

impl AppState {
    pub fn new(db: Database, mut config: Config) -> Self {
        let data_dir = config.data_dir.clone();
        config.apply_runtime_overrides(&data_dir);
        let files_dir = data_dir.join("files");
        std::fs::create_dir_all(&files_dir).expect("无法创建文件存储目录");
        let (hub, _) = broadcast::channel(256);
        Self {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(config)),
            hub,
            data_dir: Arc::new(data_dir),
            files_dir: Arc::new(files_dir),
            started_at: crate::auth::now_ms(),
        }
    }

    pub fn config_snapshot(&self) -> Config {
        self.config.read().expect("配置读写锁已失效").clone()
    }

    /// fail-fast：校验不通过直接拒绝，不静默使用旧值。
    pub fn apply_config_update(&self, req: ConfigUpdateReq) -> ApiResult<Config> {
        let mut current = self.config_snapshot();
        if let Some(v) = req.allow_registration {
            current.allow_registration = v;
        }
        if let Some(v) = req.max_file_size_mb {
            if v == 0 || v > crate::config::MAX_FILE_SIZE_MB {
                return Err(ApiError::bad_request(format!(
                    "max_file_size_mb 必须在 1~{} 之间",
                    crate::config::MAX_FILE_SIZE_MB
                )));
            }
            current.max_file_size_mb = v;
        }
        let settings = RuntimeSettings {
            allow_registration: Some(current.allow_registration),
            max_file_size_mb: Some(current.max_file_size_mb),
        };
        save_runtime_settings(&self.data_dir, &settings).map_err(ApiError::internal)?;
        *self.config.write().expect("配置读写锁已失效") = current.clone();
        Ok(current)
    }
}
