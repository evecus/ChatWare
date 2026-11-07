use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 运行期文件大小上限的允许范围（MB），必须小于 main.rs 的 HARD_BODY_LIMIT
pub const MAX_FILE_SIZE_MB: u64 = 1024;

/// Chatware 命令行参数。配置校验失败一律 fail-fast，不做静默降级。
#[derive(Parser, Debug, Clone)]
#[command(
    name = "chatware",
    version,
    about = "自托管聊天服务端（TLS 请交给反向代理）"
)]
pub struct Cli {
    /// 配置文件路径
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,
    /// 数据目录（覆盖配置文件）
    #[arg(short, long)]
    pub data_dir: Option<PathBuf>,
    /// 监听地址（覆盖配置文件）
    #[arg(long)]
    pub bind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub bind: String,
    pub data_dir: PathBuf,
    pub max_file_size_mb: u64,
    pub max_message_length: usize,
    pub allow_registration: bool,
    pub server_name: String,
    /// 反代之后对外访问的地址，仅用于管理面板展示与指纹提示
    pub public_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3000".to_string(),
            data_dir: PathBuf::from("./data"),
            max_file_size_mb: 100,
            max_message_length: 4000,
            allow_registration: true,
            server_name: "Chatware".to_string(),
            public_url: String::new(),
        }
    }
}

impl Config {
    pub fn load(cli: &Cli) -> Result<Self, String> {
        let mut cfg = if cli.config.exists() {
            let text = std::fs::read_to_string(&cli.config)
                .map_err(|e| format!("读取 {} 失败: {e}", cli.config.display()))?;
            toml::from_str::<Self>(&text)
                .map_err(|e| format!("解析 {} 失败: {e}", cli.config.display()))?
        } else {
            Self::default()
        };

        if let Some(dir) = &cli.data_dir {
            cfg.data_dir = dir.clone();
        }
        if let Some(bind) = &cli.bind {
            cfg.bind = bind.clone();
        }

        if cfg.max_file_size_mb == 0 || cfg.max_file_size_mb > MAX_FILE_SIZE_MB {
            return Err(format!("max_file_size_mb 必须在 1~{MAX_FILE_SIZE_MB} 之间"));
        }
        if cfg.max_message_length == 0 {
            return Err("max_message_length 必须大于 0".to_string());
        }
        Ok(cfg)
    }

    /// 管理面板修改的少量运行期配置，单独落盘，重启后仍然生效。
    pub fn apply_runtime_overrides(&mut self, data_dir: &Path) {
        let path = data_dir.join("settings.json");
        if let Ok(text) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<RuntimeSettings>(&text) {
                Ok(s) => {
                    if let Some(v) = s.allow_registration {
                        self.allow_registration = v;
                    }
                    if let Some(v) = s.max_file_size_mb {
                        if (1..=MAX_FILE_SIZE_MB).contains(&v) {
                            self.max_file_size_mb = v;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[chatware] {} 解析失败，已忽略: {e}", path.display());
                }
            }
        }
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb.saturating_mul(1024 * 1024)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeSettings {
    pub allow_registration: Option<bool>,
    pub max_file_size_mb: Option<u64>,
}

pub fn save_runtime_settings(data_dir: &Path, settings: &RuntimeSettings) -> Result<(), String> {
    let path = data_dir.join("settings.json");
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("写入 {} 失败: {e}", path.display()))
}
