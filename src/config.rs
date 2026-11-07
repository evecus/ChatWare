use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 运行期文件大小上限的允许范围（MB），必须小于 main.rs 的 HARD_BODY_LIMIT
pub const MAX_FILE_SIZE_MB: u64 = 1024;
/// 单条消息长度上限的允许范围（字符）
pub const MAX_MESSAGE_LENGTH: usize = 100_000;
/// 面板显示名称最大长度
const MAX_SERVER_NAME_LEN: usize = 64;
/// 对外地址最大长度
const MAX_PUBLIC_URL_LEN: usize = 512;

/// Chatware 命令行参数。只暴露启动期必须的两项，其余配置一律走内置默认值或管理面板。
#[derive(Parser, Debug, Clone)]
#[command(
    name = "chatware",
    version,
    about = "自托管聊天服务端（TLS 请交给反向代理）"
)]
pub struct Cli {
    /// 数据目录（覆盖内置默认值 ./data）
    #[arg(short, long)]
    pub data_dir: Option<PathBuf>,
    /// 监听地址（覆盖内置默认值 127.0.0.1:3000）
    #[arg(short = 'p', long)]
    pub bind: Option<String>,
}

#[derive(Debug, Clone)]
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
    /// 零配置加载：内置默认值 + 两个启动参数，统一走 validate（失败 fail-fast，不静默降级）。
    pub fn load(cli: &Cli) -> Result<Self, String> {
        let mut cfg = Self::default();
        if let Some(dir) = &cli.data_dir {
            cfg.data_dir = dir.clone();
        }
        if let Some(bind) = &cli.bind {
            cfg.bind = bind.clone();
        }
        cfg.normalize_and_validate()?;
        Ok(cfg)
    }

    /// 所有配置的唯一校验入口：启动加载、面板修改、settings.json 加载共用，避免多处规则不一致。
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_FILE_SIZE_MB).contains(&self.max_file_size_mb) {
            return Err(format!("max_file_size_mb 必须在 1~{MAX_FILE_SIZE_MB} 之间"));
        }
        if !(1..=MAX_MESSAGE_LENGTH).contains(&self.max_message_length) {
            return Err(format!(
                "max_message_length 必须在 1~{MAX_MESSAGE_LENGTH} 之间"
            ));
        }
        if self.server_name.trim().is_empty()
            || self.server_name.chars().count() > MAX_SERVER_NAME_LEN
        {
            return Err(format!(
                "server_name 必须为 1~{MAX_SERVER_NAME_LEN} 个字符且不能全为空白"
            ));
        }
        validate_public_url(&self.public_url)?;
        Ok(())
    }

    /// 先规范化字符串再校验，保证落盘值与校验依据一致。
    pub fn normalize_and_validate(&mut self) -> Result<(), String> {
        self.server_name = self.server_name.trim().to_string();
        self.public_url = self.public_url.trim().to_string();
        self.validate()
    }

    /// 管理面板修改的运行期配置，单独落盘，重启后仍然生效。
    /// 整体校验后再整体生效：settings.json 被手改坏时明确报错，不静默降级。
    pub fn apply_runtime_overrides(&mut self, data_dir: &Path) -> Result<(), String> {
        let path = data_dir.join("settings.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("读取 {} 失败: {e}", path.display())),
        };
        let s: RuntimeSettings =
            serde_json::from_str(&text).map_err(|e| format!("{} 解析失败: {e}", path.display()))?;
        let mut next = self.clone();
        if let Some(v) = s.allow_registration {
            next.allow_registration = v;
        }
        if let Some(v) = s.max_file_size_mb {
            next.max_file_size_mb = v;
        }
        if let Some(v) = s.max_message_length {
            next.max_message_length = v;
        }
        if let Some(v) = s.server_name {
            next.server_name = v;
        }
        if let Some(v) = s.public_url {
            next.public_url = v;
        }
        next.normalize_and_validate()
            .map_err(|e| format!("{} 含非法配置: {e}", path.display()))?;
        *self = next;
        Ok(())
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb.saturating_mul(1024 * 1024)
    }
}

/// public_url 会拼进 openssl 指纹命令，因此拒绝空白、控制字符与 shell 元字符。
/// 空串表示未设置，connection_info 回退为 http://<bind>。
fn validate_public_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Ok(());
    }
    if url.chars().count() > MAX_PUBLIC_URL_LEN {
        return Err(format!("public_url 不能超过 {MAX_PUBLIC_URL_LEN} 个字符"));
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| "public_url 必须以 http:// 或 https:// 开头，或留空".to_string())?;
    if rest.is_empty()
        || rest.starts_with('/')
        || url.chars().any(|c| {
            c.is_whitespace()
                || c.is_control()
                || matches!(
                    c,
                    '"' | '\'' | '\\' | '<' | '>' | '`' | '$' | ';' | '|' | '&'
                )
        })
    {
        return Err("public_url 含非法字符或缺少主机名".to_string());
    }
    Ok(())
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeSettings {
    pub allow_registration: Option<bool>,
    pub max_file_size_mb: Option<u64>,
    pub max_message_length: Option<usize>,
    pub server_name: Option<String>,
    pub public_url: Option<String>,
}

pub fn save_runtime_settings(data_dir: &Path, settings: &RuntimeSettings) -> Result<(), String> {
    let path = data_dir.join("settings.json");
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("写入 {} 失败: {e}", path.display()))
}
