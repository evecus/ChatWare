use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    #[arg(short, long, global = true)]
    pub data_dir: Option<PathBuf>,
    /// 监听地址（覆盖内置默认值 127.0.0.1:3000）
    #[arg(short = 'p', long)]
    pub bind: Option<String>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// 设置或重置 /admin 登录用的管理密码（离线操作数据库，需先停止服务）
    Passwd {
        /// 新密码，8~128 个字符
        #[arg(long)]
        password: String,
    },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub data_dir: PathBuf,
    pub max_file_size_mb: u64,
    pub max_message_length: usize,
    pub allow_registration: bool,
    pub server_name: String,
    /// 反代之后对外访问的地址，供管理面板展示与客户端连接页复制
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

    /// 所有配置的唯一校验入口：启动加载、面板修改、数据库设置加载共用，避免多处规则不一致。
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

    /// 用数据库 settings 表里的运行期设置覆盖默认值。
    /// 整体校验后再整体生效：库里出现非法值时明确报错，不静默降级、也不会只生效一半。
    pub fn apply_runtime_overrides(&mut self, settings: &RuntimeSettings) -> Result<(), String> {
        let mut next = self.clone();
        if let Some(v) = settings.allow_registration {
            next.allow_registration = v;
        }
        if let Some(v) = settings.max_file_size_mb {
            next.max_file_size_mb = v;
        }
        if let Some(v) = settings.max_message_length {
            next.max_message_length = v;
        }
        if let Some(v) = &settings.server_name {
            next.server_name = v.clone();
        }
        if let Some(v) = &settings.public_url {
            next.public_url = v.clone();
        }
        next.normalize_and_validate()
            .map_err(|e| format!("数据库 settings 表含非法配置: {e}"))?;
        *self = next;
        Ok(())
    }

    /// 面板修改后的完整快照，用于写回数据库。
    pub fn runtime_settings(&self) -> RuntimeSettings {
        RuntimeSettings {
            allow_registration: Some(self.allow_registration),
            max_file_size_mb: Some(self.max_file_size_mb),
            max_message_length: Some(self.max_message_length),
            server_name: Some(self.server_name.clone()),
            public_url: Some(self.public_url.clone()),
        }
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb.saturating_mul(1024 * 1024)
    }
}

/// public_url 用于客户端地址展示与复制，拒绝空白、控制字符与超长；空串表示未设置。
/// 未设置时 connection_info 回退为 http://<bind>。
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
        || url.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("public_url 含非法字符或缺少主机名".to_string());
    }
    Ok(())
}

/// 运行期设置：字段为 None 表示数据库中未保存，沿用内置默认值。
/// 直接以固定 key + 纯文本值存入 redb，不经过 JSON 序列化。
#[derive(Debug, Default, Clone)]
pub struct RuntimeSettings {
    pub allow_registration: Option<bool>,
    pub max_file_size_mb: Option<u64>,
    pub max_message_length: Option<usize>,
    pub server_name: Option<String>,
    pub public_url: Option<String>,
}

impl RuntimeSettings {
    /// 编码为 (key, value) 列表；None 字段不写入。
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::with_capacity(5);
        if let Some(v) = self.allow_registration {
            out.push(("allow_registration", v.to_string()));
        }
        if let Some(v) = self.max_file_size_mb {
            out.push(("max_file_size_mb", v.to_string()));
        }
        if let Some(v) = self.max_message_length {
            out.push(("max_message_length", v.to_string()));
        }
        if let Some(v) = &self.server_name {
            out.push(("server_name", v.clone()));
        }
        if let Some(v) = &self.public_url {
            out.push(("public_url", v.clone()));
        }
        out
    }

    /// 从 (key, value) 列表解码；未知 key 或非法值直接报错（fail-fast）。
    pub fn from_pairs(pairs: &[(String, String)]) -> Result<Self, String> {
        let mut out = Self::default();
        for (key, value) in pairs {
            match key.as_str() {
                "allow_registration" => {
                    out.allow_registration = Some(match value.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(format!("settings.allow_registration 非法值: {value}")),
                    });
                }
                "max_file_size_mb" => {
                    out.max_file_size_mb = Some(
                        value
                            .parse::<u64>()
                            .map_err(|_| format!("settings.max_file_size_mb 非法值: {value}"))?,
                    );
                }
                "max_message_length" => {
                    out.max_message_length = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("settings.max_message_length 非法值: {value}"))?,
                    );
                }
                "server_name" => out.server_name = Some(value.clone()),
                "public_url" => out.public_url = Some(value.clone()),
                other => return Err(format!("settings 表含未知 key: {other}")),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn settings_roundtrip() {
        let original = Config {
            allow_registration: false,
            max_file_size_mb: 256,
            max_message_length: 5,
            server_name: "内测大厅".to_string(),
            public_url: "https://chat.example.com".to_string(),
            ..Config::default()
        };
        let decoded = RuntimeSettings::from_pairs(&pairs(
            &original
                .runtime_settings()
                .to_pairs()
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
        ))
        .unwrap();
        let mut cfg = Config::default();
        cfg.apply_runtime_overrides(&decoded).unwrap();
        assert!(!cfg.allow_registration);
        assert_eq!(cfg.max_file_size_mb, 256);
        assert_eq!(cfg.max_message_length, 5);
        assert_eq!(cfg.server_name, "内测大厅");
        assert_eq!(cfg.public_url, "https://chat.example.com");
    }

    #[test]
    fn settings_reject_illegal_values() {
        assert!(RuntimeSettings::from_pairs(&pairs(&[("allow_registration", "yes")])).is_err());
        assert!(RuntimeSettings::from_pairs(&pairs(&[("max_file_size_mb", "abc")])).is_err());
        assert!(RuntimeSettings::from_pairs(&pairs(&[("max_message_length", "-1")])).is_err());
        assert!(RuntimeSettings::from_pairs(&pairs(&[("unknown_key", "1")])).is_err());
    }

    #[test]
    fn settings_empty_uses_defaults() {
        let mut cfg = Config::default();
        cfg.apply_runtime_overrides(&RuntimeSettings::default())
            .unwrap();
        assert_eq!(cfg.server_name, "Chatware");
        assert_eq!(cfg.max_file_size_mb, 100);
    }
}
