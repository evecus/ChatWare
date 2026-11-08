//! 登录与敏感接口的防爆破限流。
//!
//! 设计：
//! - 内存态滑动窗口：统计每个 key 在窗口期内的失败次数，超过阈值后按递增时长锁定。
//! - 双层 key：主 key 是「客户端 IP」（优先取反向代理设置的 X-Real-IP，其次
//!   X-Forwarded-For 第一段，都没有则退回 socket 对端 IP）；副 key 恒为 socket
//!   对端 IP。主 key 防止单个来源爆破，副 key 兜底：直连时伪造 X-Forwarded-For
//!   轮换身份也绕不过副 key 的更高阈值。
//! - 只在进程内存中计数：重启即清零，换取零磁盘写放大；锁定时长递增封顶。

use crate::error::ApiError;
use axum::http::HeaderMap;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// 失败统计窗口：10 分钟。
const WINDOW_MS: i64 = 10 * 60 * 1000;
/// 单客户端 IP 在窗口内允许的失败次数（登录 / 注册 / WS 鉴权各自独立计数）。
pub const CLIENT_MAX_FAILURES: u32 = 10;
/// 单个 socket 对端 IP 在窗口内允许的失败次数。
/// 走反代时它等于反代地址、由全体用户共享，因此阈值放宽一个数量级。
pub const PEER_MAX_FAILURES: u32 = 100;
/// 首次锁定 1 分钟，之后每次触发翻倍，封顶 15 分钟。
const LOCKOUT_BASE_MS: i64 = 60 * 1000;
const LOCKOUT_MAX_MS: i64 = 15 * 60 * 1000;

#[derive(Default)]
struct Entry {
    /// 窗口内的失败时间戳（毫秒）
    failures: Vec<i64>,
    /// 锁定截止时间（毫秒）；0 表示未锁定
    locked_until: i64,
    /// 历史触发锁定的次数，决定下次锁定时长
    lockouts: u32,
}

/// 全局限流器，随 AppState 共享。
#[derive(Default)]
pub struct RateLimiter {
    entries: Mutex<HashMap<String, Entry>>,
}

impl RateLimiter {
    fn full_key(scope: &str, id: &str) -> String {
        format!("{scope}:{id}")
    }

    /// 超过阈值即锁定（无论密码对错），返回 429 与剩余秒数。
    pub fn check(&self, scope: &str, id: &str) -> Result<(), ApiError> {
        let now = crate::auth::now_ms();
        let mut map = self.entries.lock().expect("限流器锁已失效");
        let full = Self::full_key(scope, id);
        let keep = match map.get_mut(&full) {
            Some(entry) => {
                entry.failures.retain(|t| now - *t < WINDOW_MS);
                if entry.locked_until > now {
                    let secs = (entry.locked_until - now + 999) / 1000;
                    return Err(ApiError::too_many_requests(
                        format!("尝试过于频繁，已被临时锁定，请 {secs} 秒后再试"),
                        secs,
                    ));
                }
                // 锁定期已过且窗口内无失败记录很久了，把递增记忆一并清掉
                let stale = entry.failures.is_empty() && now - entry.locked_until > WINDOW_MS;
                !stale
            }
            None => false,
        };
        if !keep {
            map.remove(&full);
        }
        Ok(())
    }

    fn record_failure(&self, scope: &str, id: &str, max: u32) {
        let now = crate::auth::now_ms();
        let mut map = self.entries.lock().expect("限流器锁已失效");
        let entry = map.entry(Self::full_key(scope, id)).or_default();
        entry.failures.retain(|t| now - *t < WINDOW_MS);
        entry.failures.push(now);
        if entry.failures.len() as u32 >= max {
            let exp = entry.lockouts.min(4);
            let dur = std::cmp::min(LOCKOUT_BASE_MS << exp, LOCKOUT_MAX_MS);
            entry.locked_until = now + dur;
            entry.lockouts = entry.lockouts.saturating_add(1);
            entry.failures.clear();
        }
    }

    fn clear(&self, scope: &str, id: &str) {
        self.entries
            .lock()
            .expect("限流器锁已失效")
            .remove(&Self::full_key(scope, id));
    }

    /// 双层校验：主 key（客户端 IP）与副 key（对端 IP）任一被锁即拒绝。
    pub fn verify(&self, scope: &str, primary: &str, peer: &str) -> Result<(), ApiError> {
        self.check(scope, primary)?;
        if peer != primary {
            self.check(scope, peer)?;
        }
        Ok(())
    }

    /// 双层记失败：主 key 用严格阈值，副 key 用宽松阈值兜底。
    pub fn fail(&self, scope: &str, primary: &str, peer: &str) {
        self.record_failure(scope, primary, CLIENT_MAX_FAILURES);
        if peer != primary {
            self.record_failure(scope, peer, PEER_MAX_FAILURES);
        }
    }

    /// 成功后清零，正常用户不会被历史失败连坐。
    pub fn succeed(&self, scope: &str, primary: &str, peer: &str) {
        self.clear(scope, primary);
        self.clear(scope, peer);
    }
}

/// 提取限流 key：主 key 优先 X-Real-IP（推荐反代覆盖式设置），其次
/// X-Forwarded-For 第一段，最后退回 socket 对端 IP。副 key 恒为对端 IP。
pub fn client_keys(headers: &HeaderMap, peer: SocketAddr) -> (String, String) {
    let peer_ip = peer.ip().to_string();
    let real = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 64);
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 64);
    let primary = real.or(forwarded).unwrap_or(&peer_ip).to_string();
    (primary, peer_ip)
}

/// AppState 持有的共享句柄。
pub type SharedRateLimiter = Arc<RateLimiter>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 锁定后拒绝并携带重试秒数() {
        let rl = RateLimiter::default();
        for _ in 0..CLIENT_MAX_FAILURES {
            rl.record_failure("user-login", "1.2.3.4", CLIENT_MAX_FAILURES);
        }
        let err = rl.check("user-login", "1.2.3.4").unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(err.retry_after_secs() >= 1);
    }

    #[test]
    fn 成功清零后不再锁定() {
        let rl = RateLimiter::default();
        for _ in 0..CLIENT_MAX_FAILURES - 1 {
            rl.record_failure("user-login", "a", CLIENT_MAX_FAILURES);
        }
        rl.succeed("user-login", "a", "a");
        rl.check("user-login", "a").unwrap();
        // 剩余阈值从零重新计数
        for _ in 0..CLIENT_MAX_FAILURES {
            rl.record_failure("user-login", "a", CLIENT_MAX_FAILURES);
        }
        assert!(rl.check("user-login", "a").is_err());
    }

    #[test]
    fn 不同场景互不影响() {
        let rl = RateLimiter::default();
        for _ in 0..CLIENT_MAX_FAILURES {
            rl.record_failure("user-login", "x", CLIENT_MAX_FAILURES);
        }
        rl.check("user-login", "x").unwrap_err();
        rl.check("admin-login", "x").unwrap();
    }
}
