/// 两个面板共用的样式表。白色背景 + 现代灵动的卡片与渐变，
/// 聊天区采用微信风格的左右气泡，并完整适配桌面与移动端。
pub const CSS: &str = r##"
:root{--brand:#5b6cff;--brand2:#8b5cf6;--ink:#1f2430;--muted:#8a91a3;--line:#eceef5;--bg:#f7f8fc;--danger:#ef4444;--ok:#10b981;--shadow:0 12px 32px rgba(40,50,90,.09)}
*{box-sizing:border-box}
html,body{height:100%}
body{margin:0;background:#fff;color:var(--ink);font:14px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI","PingFang SC","Microsoft YaHei",sans-serif;-webkit-font-smoothing:antialiased}
a{color:var(--brand);text-decoration:none}
button,input,textarea,select{font:inherit}
.hidden{display:none!important}
.spacer{flex:1}
.mono{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:12px}

/* ---------- 通用组件 ---------- */
.input{width:100%;border:1.5px solid var(--line);background:#fff;border-radius:12px;padding:11px 13px;outline:none;transition:.2s;color:var(--ink)}
.input:focus{border-color:var(--brand);box-shadow:0 0 0 4px rgba(91,108,255,.12)}
.btn{border:0;border-radius:12px;padding:11px 18px;cursor:pointer;font-weight:600;transition:.2s;display:inline-flex;align-items:center;justify-content:center;gap:8px;white-space:nowrap}
.btn-primary{background:linear-gradient(135deg,var(--brand),var(--brand2));color:#fff;box-shadow:0 8px 20px rgba(91,108,255,.28)}
.btn-primary:hover{transform:translateY(-2px);box-shadow:0 12px 26px rgba(91,108,255,.36)}
.btn-ghost{background:var(--bg);color:var(--ink)}
.btn-ghost:hover{background:#eef0f8}
.btn-danger{background:#fef2f2;color:var(--danger)}
.btn-sm{padding:7px 12px;font-size:12.5px;border-radius:9px}
.btn-block{width:100%}
.tabs{display:flex;gap:6px;background:var(--bg);padding:5px;border-radius:12px;margin-bottom:22px}
.tabs button{flex:1;border:0;background:transparent;padding:9px;border-radius:9px;color:var(--muted);cursor:pointer;transition:.2s;font-weight:600}
.tabs button.active{background:#fff;color:var(--ink);box-shadow:0 2px 8px rgba(30,40,80,.08)}
.field{margin-bottom:14px}
.field label{display:block;font-size:12.5px;color:var(--muted);margin-bottom:6px;font-weight:600}
.hint{font-size:12px;color:var(--muted);margin-top:12px;text-align:center}
.err{background:#fef2f2;color:#b91c1c;padding:10px 12px;border-radius:10px;font-size:12.5px;margin-bottom:14px}
.tag{display:inline-block;padding:3px 9px;border-radius:999px;font-size:11.5px;font-weight:600;background:#eef0ff;color:#4f46e5}
.tag.gray{background:#f1f3f7;color:#7b8296}.tag.red{background:#fef2f2;color:#dc2626}.tag.green{background:#ecfdf5;color:#059669}
.toast{position:fixed;bottom:26px;left:50%;transform:translateX(-50%) translateY(20px);background:var(--ink);color:#fff;padding:11px 20px;border-radius:12px;font-size:13px;opacity:0;pointer-events:none;transition:.25s;z-index:99;box-shadow:0 10px 30px rgba(0,0,0,.2)}
.toast.show{opacity:1;transform:translateX(-50%)}
.card{background:#fff;border:1px solid var(--line);border-radius:18px;padding:22px;box-shadow:var(--shadow);margin-bottom:18px}
.card h3{margin:0 0 16px;font-size:15.5px}
.row{display:flex;gap:10px;align-items:center;flex-wrap:wrap}
.row .input{flex:1;min-width:120px}

/* ---------- 登录 / 注册 ---------- */
.auth-wrap{min-height:100dvh;display:grid;place-items:center;padding:24px;background:radial-gradient(1000px 500px at 15% -10%,#eef0ff,transparent),radial-gradient(900px 500px at 100% 100%,#f3eaff,transparent),#fff}
.auth-card{width:min(420px,100%);background:#fff;border:1px solid var(--line);border-radius:22px;box-shadow:var(--shadow);padding:32px;animation:rise .35s ease}
@keyframes rise{from{opacity:0;transform:translateY(12px)}to{opacity:1;transform:none}}
.brand{display:flex;align-items:center;gap:12px}
.brand-mark{width:42px;height:42px;border-radius:13px;background:linear-gradient(135deg,var(--brand),var(--brand2));display:grid;place-items:center;color:#fff;font-weight:800;font-size:20px;box-shadow:0 8px 18px rgba(91,108,255,.35)}
.brand h1{font-size:20px;margin:0;letter-spacing:.2px}
.brand-sub{color:var(--muted);font-size:12.5px;margin:2px 0 24px 54px}

/* ---------- 管理面板 ---------- */
.topbar{height:64px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:14px;padding:0 24px;background:rgba(255,255,255,.9);backdrop-filter:blur(12px);position:sticky;top:0;z-index:20}
.topbar .brand-mark{width:34px;height:34px;border-radius:10px;font-size:16px}
.topbar .title{font-weight:700;font-size:15px}
.wrap{max-width:1120px;margin:0 auto;padding:26px 24px 60px}
.page-title{font-size:22px;font-weight:750;margin:0 0 4px}
.page-sub{color:var(--muted);margin:0 0 22px;font-size:13px}
.navtabs{display:flex;gap:8px;flex-wrap:wrap;margin-bottom:22px}
.navtabs button{border:1px solid var(--line);background:#fff;color:var(--muted);padding:8px 16px;border-radius:999px;cursor:pointer;font-weight:600;transition:.2s}
.navtabs button.active{background:var(--ink);color:#fff;border-color:var(--ink)}
.grid{display:grid;grid-template-columns:repeat(4,1fr);gap:16px;margin-bottom:22px}
.stat{background:#fff;border:1px solid var(--line);border-radius:18px;padding:20px;box-shadow:var(--shadow);transition:.2s}
.stat:hover{transform:translateY(-3px)}
.stat .label{color:var(--muted);font-size:12.5px}
.stat .value{font-size:28px;font-weight:750;margin-top:6px;letter-spacing:-.5px}
.table-scroll{overflow-x:auto}
table{width:100%;border-collapse:collapse;font-size:13px}
th{text-align:left;color:var(--muted);font-weight:600;font-size:12px;padding:10px 12px;border-bottom:1px solid var(--line);white-space:nowrap}
td{padding:12px;border-bottom:1px solid var(--line);vertical-align:middle}
tr:last-child td{border-bottom:0}
.switch{display:flex;align-items:center;gap:10px;cursor:pointer;font-weight:600}
.switch input{appearance:none;width:42px;height:24px;border-radius:999px;background:#d8dce8;position:relative;transition:.2s;cursor:pointer;margin:0}
.switch input:checked{background:var(--brand)}
.switch input::after{content:"";position:absolute;top:3px;left:3px;width:18px;height:18px;border-radius:50%;background:#fff;transition:.2s;box-shadow:0 1px 3px rgba(0,0,0,.2)}
.switch input:checked::after{transform:translateX(18px)}

/* ---------- 聊天（微信风格） ---------- */
.chat-app{display:flex;height:100dvh;background:#fff;overflow:hidden}
.sidebar{width:286px;flex:0 0 286px;border-right:1px solid var(--line);display:flex;flex-direction:column;background:#fbfcff;z-index:31}
.side-head{padding:20px;border-bottom:1px solid var(--line)}
.side-head .label{font-size:11.5px;color:var(--muted);font-weight:600;margin-bottom:6px}
.side-head .addr{font-size:12.5px;word-break:break-all;font-weight:600}
.side-body{flex:1;overflow-y:auto;padding:12px}
.side-body .caption{font-size:11.5px;color:var(--muted);font-weight:700;padding:6px 10px;letter-spacing:.5px}
.file-item{display:flex;gap:10px;align-items:center;padding:10px;border-radius:12px;cursor:pointer;transition:.2s}
.file-item:hover{background:#eef1ff}
.file-ico{width:36px;height:36px;border-radius:10px;background:#eef0ff;color:var(--brand);display:grid;place-items:center;font-size:15px;font-weight:700;flex:0 0 36px}
.file-name{font-size:13px;font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:180px}
.file-meta{font-size:11.5px;color:var(--muted)}
.side-foot{padding:16px;border-top:1px solid var(--line);display:flex;align-items:center;gap:10px}
.side-foot .who{flex:1;min-width:0;font-size:13px;font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.chat-main{flex:1;display:flex;flex-direction:column;min-width:0}
.chat-head{height:62px;flex:0 0 62px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:12px;padding:0 20px;background:#fff}
.chat-head h2{font-size:16px;margin:0;font-weight:700}
.chat-head .sub{font-size:11.5px;color:var(--muted)}
.status-dot{width:8px;height:8px;border-radius:50%;background:#cbd5e1;display:inline-block;margin-right:6px}
.status-dot.on{background:var(--ok);box-shadow:0 0 0 4px rgba(16,185,129,.15)}
.messages{flex:1;overflow-y:auto;background:#ededed;padding:22px 18px}
.messages-inner{max-width:880px;margin:0 auto}
.load-more{text-align:center;margin-bottom:12px}
.msg{display:flex;gap:10px;align-items:flex-start;margin-bottom:18px;animation:pop .22s ease}
@keyframes pop{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
.msg.me{flex-direction:row-reverse}
.avatar{width:40px;height:40px;flex:0 0 40px;border-radius:9px;color:#fff;display:grid;place-items:center;font-weight:700;font-size:15px;user-select:none}
.msg-body{max-width:min(70%,560px);min-width:0}
.msg.me .msg-body{display:flex;flex-direction:column;align-items:flex-end}
.mname{font-size:11.5px;color:#9299a8;margin:0 2px 5px}
.bubble{position:relative;background:#fff;border-radius:8px;padding:10px 13px;box-shadow:0 1px 2px rgba(0,0,0,.06);word-break:break-word;white-space:pre-wrap;line-height:1.55;font-size:14.5px;display:inline-block;text-align:left}
.msg.me .bubble{background:#95ec69}
.bubble::before{content:"";position:absolute;top:13px;left:-4px;width:9px;height:9px;background:#fff;transform:rotate(45deg)}
.msg.me .bubble::before{left:auto;right:-4px;background:#95ec69}
.file-bubble{display:flex;align-items:center;gap:12px;min-width:220px;cursor:pointer}
.file-bubble .fname{font-weight:600;font-size:13.5px;max-width:230px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.file-bubble .fsize{font-size:11.5px;color:#8a91a3}
.dl{width:34px;height:34px;border-radius:9px;background:rgba(91,108,255,.12);color:var(--brand);display:grid;place-items:center;font-weight:700;flex:0 0 34px}
.composer{flex:0 0 auto;border-top:1px solid #e3e5ea;background:#f7f7f7;padding:12px 18px 16px}
.composer-inner{max-width:880px;margin:0 auto;display:flex;gap:10px;align-items:flex-end}
.composer textarea{flex:1;resize:none;border:1.5px solid #e3e5ea;background:#fff;border-radius:12px;padding:11px 13px;outline:none;min-height:44px;max-height:120px;line-height:1.5}
.composer textarea:focus{border-color:var(--brand)}
.icon-btn{width:44px;height:44px;flex:0 0 44px;border:1.5px solid #e3e5ea;border-radius:12px;background:#fff;cursor:pointer;font-size:19px;color:#6b7280;transition:.2s}
.icon-btn:hover{color:var(--brand);border-color:var(--brand)}
.send-btn{height:44px;padding:0 22px}
.mobile-only{display:none}
.sheet-mask{position:fixed;inset:0;background:rgba(15,20,35,.35);z-index:30}

@media(max-width:820px){
  .sidebar{position:fixed;left:0;top:0;bottom:0;transform:translateX(-100%);transition:.25s;box-shadow:var(--shadow)}
  .sidebar.open{transform:none}
  .mobile-only{display:inline-flex}
  .grid{grid-template-columns:repeat(2,1fr)}
  .wrap{padding:18px 14px 50px}
  .topbar{padding:0 14px}
  .messages{padding:16px 10px}
  .msg-body{max-width:78%}
  .composer{padding:10px}
  .auth-card{padding:24px}
  th,td{padding:9px 8px}
}
@media(max-width:480px){
  .grid{gap:10px}.stat{padding:14px}.stat .value{font-size:22px}
  .card{padding:16px}.send-btn{padding:0 14px}
}
"##;
