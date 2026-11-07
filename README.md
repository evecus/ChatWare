# Chatware

自托管聊天工具。**Rust 单二进制服务端**负责账号鉴权、邀请码、消息存储、文件收发，并内置两个 Web 面板；客户端有两种形态：**Kotlin Compose 安卓 APK** 和服务端自带的 **/user 面板**。两者复用同一套 API。

- 服务端**只监听纯 HTTP**，TLS 域名证书由 Caddy / Nginx 等反向代理提供。
- 聊天模型为**单一公共大厅**，WebSocket 实时广播 + 历史分页。
- 聊天界面为**微信风格**：自己绿色气泡靠右、他人白色气泡靠左，每条消息都显示用户名与头像。
- 每个账号都有**用户名和密码**，注册必须带管理员生成的**邀请码**。

## 架构

| 组件 | 技术 | 说明 |
|---|---|---|
| 服务端 | Rust + Axum + tokio | 单二进制，`/api/*` REST + `/api/ws` WebSocket |
| 存储 | redb 单文件 + `data/files/` | 纯 Rust 无 C 依赖，musl 静态编译干净 |
| 鉴权 | argon2 + 有状态 token | 服务端可吊销，禁用/删除账号即时生效 |
| /admin、/user 面板 | **由 `src/web/*.rs` 生成** | 白色背景、现代灵动、桌面/移动响应式；无独立前端工程、无 Node 构建 |
| 安卓客户端 | Kotlin + Compose + OkHttp | 仅 arm64-v8a，R8 精简，微信风格聊天 |
| 加密传输 | 反向代理 TLS | 真实域名证书，客户端走系统默认 CA 校验，无需证书配置 |

## 快速开始

**零配置运行**：程序不读取任何配置文件，全部用内置默认值，其余参数在管理面板里改。

```bash
./chatware -p 127.0.0.1:3000 -d /opt/chatware/data
```

| 参数 | 说明 |
|---|---|
| `-p` / `--bind <地址>` | 监听地址，默认 `127.0.0.1:3000` |
| `-d` / `--data-dir <目录>` | 数据目录，默认 `./data` |

只有这两个启动参数。`bind` 与 `data_dir` 之外的配置项一律使用内置默认值，需要调整请到 `/admin → 设置` 修改（立即生效并写入 `data/settings.json`，重启保留）。

首次启动会在**日志**打印初始管理员密码，同时写入 `data/admin.txt`：

```
Chatware 初始管理员已创建
用户名: admin
密码  : XXXXXXXXXXXXXXXX
首次登录 /admin 后必须修改密码
```

1. 浏览器打开 `http://127.0.0.1:3000/admin`，用初始密码登录并**强制改密**。
2. 在「设置」页配置服务端名称、对外地址、注册开关、文件大小与消息长度上限（立即生效并持久化）。
3. 在「邀请码」页生成邀请码（可设可用次数、有效小时、备注）。
4. 打开 `/user`，或把地址复制到安卓客户端，用邀请码注册账号后即可聊天、发文件。

> `data/settings.json` 由管理面板维护，不要手改；其内容优先级高于内置默认值与命令行，含非法值时会明确报错退出（fail-fast），不会静默降级。

## TLS 与反向代理

本程序不处理证书。Caddy 一行即可（自动支持 WebSocket）：

```caddyfile
chat.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

Nginx 需要显式转发 Upgrade 头，否则 `/api/ws` 无法连接：

```nginx
location / {
    proxy_pass http://127.0.0.1:3000;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    client_max_body_size 200m;   # 需 >= max_file_size_mb
}
```

反代部署时在 `/admin → 设置 → 对外地址` 填入反代后的地址（如 `https://chat.example.com`），`/admin → 客户端连接` 页会据此给出可直接复制的地址。留空则回退为 `http://<bind>`。

## 客户端连接

| 项 | 说明 |
|---|---|
| 服务端地址 | 从 `/admin → 客户端连接` 复制（如 `https://chat.example.com`），填到安卓客户端的「服务端地址」 |
| 证书 | 反向代理提供真实域名证书，客户端走系统默认 CA 校验，**无需填写指纹、也没有跳过校验开关** |
| 明文 HTTP | 允许（`usesCleartextTraffic`），局域网直连 `http://IP:3000` 也能用 |

安卓客户端把**服务端地址**与**登录 / 注册**合在同一页：填好地址，切换登录或注册标签，输入用户名、密码（注册还需邀请码）即可。地址保存在本地，下次打开自动填入；聊天页右上角「切换」可回到该页改地址。

## 主要接口

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/register` | 用户名 + 密码 + 邀请码 |
| POST | `/api/login` / `/api/logout` | 签发 / 吊销 token |
| GET | `/api/me`、`/api/users` | 当前账号 / 用户列表 |
| POST | `/api/change-password` | 改密（清除强制改密标记） |
| GET | `/api/messages?before=&limit=` | 历史分页，`before` 传最早一条的 `key` |
| WS | `/api/ws?token=` | 收发文字/文件消息，服务端落库后广播 |
| POST / GET | `/api/files`、`/api/files/:id` | 流式上传 / 下载 |
| GET | `/api/admin/summary` | 用户数、消息数、文件数 |
| GET/POST/DELETE | `/api/admin/invites` | 邀请码管理 |
| GET/POST/DELETE | `/api/admin/users` | 禁用 / 启用 / 删除账号 |
| GET/POST | `/api/admin/config` | 5 项运行参数（注册开关、文件大小、消息长度、名称、对外地址），立即生效并持久化 |
| GET | `/api/admin/connection-info` | 对外地址 |

所有响应错误统一为 `{ "error": "..." }`；除登录、注册、`/api/info` 外都需要 `Authorization: Bearer <token>`。

## 配置项

启动时由内置默认值与命令行 `-p`/`-d` 决定；带「面板可改」的项还能在 `/admin → 设置` 里热改并写入 `data/settings.json`（重启保留，优先级最高）。

| 配置 | 默认值 | 面板可改 | 说明 |
|---|---|---|---|
| `bind` | `127.0.0.1:3000` | ❌ | 监听地址，启动参数 `-p` 决定 |
| `data_dir` | `./data` | ❌ | 数据库、文件、管理员密码，启动参数 `-d` 决定 |
| `max_file_size_mb` | `100` | ✅ | 单文件上限（1~1024），超限立即删除临时文件 |
| `max_message_length` | `4000` | ✅ | 单条消息字符数（1~100000） |
| `allow_registration` | `true` | ✅ | 是否开放注册 |
| `server_name` | `Chatware` | ✅ | 面板标题（1~64 字符） |
| `public_url` | 空 | ✅ | 反代后的对外地址，须 `http(s)://` 开头；留空回退 `http://<bind>` |

面板输入同样走与启动加载**同一套校验**，非法值直接拒绝（fail-fast），不会静默使用旧值。

## 构建与 CI

两个 workflow 都在 `v*` tag 触发并自动发布到同一个 GitHub Release（两者共用 `concurrency` 组串行追加，避免写冲突）。

| workflow | 目标 | 关键步骤 |
|---|---|---|
| `server.yml` | `x86_64-unknown-linux-musl` 单二进制 | `fmt --check` → `clippy --locked -D warnings` → 依赖树守卫（禁止 `aws-lc-rs`）→ release 编译 → `strip` → tar.gz |
| `android.yml` | `arm64-v8a` release APK | JDK 17 + SDK 35 → secrets 解码 keystore → `assembleRelease` → R8 + `shrinkResources` + 单 ABI → `apksigner verify` |

**签名 Secrets**（`Settings → Secrets and variables → Actions`）：

| Secret | 内容 |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | `base64 -w0 release.jks`（Git Bash 里执行） |
| `ANDROID_KEYSTORE_PASSWORD` | 密钥库口令 |
| `ANDROID_KEY_ALIAS` | 别名 |
| `ANDROID_KEY_PASSWORD` | 密钥口令 |

生成 keystore：

```bash
keytool -genkeypair -v -keystore release.jks -alias chatware \
  -keyalg RSA -keysize 2048 -validity 10000
base64 -w0 release.jks
```

未配置签名 secrets 时 CI 会用 debug 签名兜底，保证仍能产出可安装 APK（日志给出 warning）。本地编译：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --locked --target x86_64-unknown-linux-musl
cd android && ./gradlew :app:assembleRelease
```

## 目录结构

```
chatware/
├── Cargo.toml / Cargo.lock
├── src/
│   ├── main.rs                 # CLI、路由、启动
│   ├── config.rs / db.rs / auth.rs / state.rs
│   ├── api/{auth_api,chat_api,file_api,admin_api}.rs
│   └── web/{mod,assets,admin,user}.rs   # 两个面板的 HTML/CSS/JS 全部是 Rust 常量
├── android/app/src/main/       # Compose 客户端 + 自适应图标
└── .github/workflows/{server,android}.yml
```

## 安全说明

- 密码 argon2 哈希；token 服务端有状态，登出/禁用/删除账号立即失效；邀请码在同一写事务内核销，不会并发超发。
- 文件下载需登录；文件名经清洗与 RFC 5987 编码，避免路径穿越与响应头注入。
- 传输安全由反代 TLS 提供，客户端使用系统默认 CA 校验；**没有端到端加密**，服务端能看到明文聊天内容。
- 直连 HTTP 仅适合内网；公网部署请务必配置反向代理 TLS。
