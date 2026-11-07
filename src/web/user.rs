/// /user 客户端面板：页面结构、样式引用与全部交互逻辑都由本 .rs 文件生成。
/// 聊天界面为微信风格：自己绿色气泡靠右、他人白色气泡靠左，均显示用户名与头像。
pub const PAGE: &str = r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>Chatware</title>
<link rel="stylesheet" href="/assets/style.css">
</head>
<body>

<div id="authView" class="auth-wrap">
  <div class="auth-card">
    <div class="brand"><div class="brand-mark">C</div><h1 id="brandName">Chatware</h1></div>
    <p class="brand-sub">连接服务端，开始聊天</p>
    <div class="tabs">
      <button type="button" id="tabLogin" class="active">登录</button>
      <button type="button" id="tabRegister">注册</button>
    </div>

    <div id="loginErr" class="err hidden"></div>
    <form id="loginForm">
      <div class="field"><label>用户名</label><input class="input" id="loginUser" autocomplete="username" required></div>
      <div class="field"><label>密码</label><input class="input" id="loginPass" type="password" autocomplete="current-password" required></div>
      <button class="btn btn-primary btn-block" type="submit">登录</button>
    </form>

    <div id="regErr" class="err hidden"></div>
    <form id="registerForm" class="hidden">
      <div class="field"><label>用户名</label><input class="input" id="regUser" autocomplete="username" required><div class="hint" style="text-align:left">2~24 个字符，支持中文，将显示在聊天界面</div></div>
      <div class="field"><label>密码</label><input class="input" id="regPass" type="password" autocomplete="new-password" required></div>
      <div class="field"><label>确认密码</label><input class="input" id="regPass2" type="password" autocomplete="new-password" required></div>
      <div class="field"><label>注册邀请码</label><input class="input mono" id="regCode" placeholder="由管理员在 /admin 生成" required></div>
      <button class="btn btn-primary btn-block" type="submit">注册</button>
    </form>

    <p class="hint">服务端地址：<span id="srvAddr" class="mono"></span></p>
    <p class="hint"><button type="button" id="copyAddr" class="btn btn-ghost btn-sm">复制地址到安卓客户端</button></p>
  </div>
</div>

<div id="changeView" class="auth-wrap hidden">
  <div class="auth-card">
    <div class="brand"><div class="brand-mark">C</div><h1>修改初始密码</h1></div>
    <p class="brand-sub">首次登录必须修改密码后才能继续使用</p>
    <div id="changeErr" class="err hidden"></div>
    <form id="changeForm">
      <div class="field"><label>原密码</label><input class="input" id="chgOld" type="password" required></div>
      <div class="field"><label>新密码（至少 8 位）</label><input class="input" id="chgNew" type="password" required></div>
      <div class="field"><label>确认新密码</label><input class="input" id="chgNew2" type="password" required></div>
      <button class="btn btn-primary btn-block" type="submit">保存并进入</button>
    </form>
    <p class="hint"><button type="button" id="chgLogout" class="btn btn-ghost btn-sm">退出登录</button></p>
  </div>
</div>

<div id="appView" class="chat-app hidden">
  <div id="sheetMask" class="sheet-mask hidden"></div>
  <aside id="sidebar" class="sidebar">
    <div class="side-head">
      <div class="label">当前服务端</div>
      <div class="addr mono" id="srvAddr2"></div>
      <button type="button" id="copyAddr2" class="btn btn-ghost btn-sm" style="margin-top:10px">复制地址到安卓端</button>
    </div>
    <div class="side-body">
      <div class="caption">共享文件 · 点击下载</div>
      <div id="fileList"></div>
    </div>
    <div class="side-foot"><div class="who" id="meName"></div><button type="button" id="logoutBtn" class="btn btn-ghost btn-sm">退出</button></div>
  </aside>

  <div class="chat-main">
    <div class="chat-head">
      <button type="button" id="menuBtn" class="icon-btn mobile-only">&#9776;</button>
      <div>
        <h2 id="chatTitle">Chatware</h2>
        <div class="sub"><span id="statusDot" class="status-dot"></span><span id="statusText">连接中</span></div>
      </div>
      <div class="spacer"></div>
      <button type="button" id="copyAddr3" class="btn btn-ghost btn-sm">复制地址</button>
    </div>

    <div id="messages" class="messages">
      <div class="load-more hidden" id="loadMoreBox"><button type="button" id="loadMore" class="btn btn-ghost btn-sm">加载更早的消息</button></div>
      <div class="messages-inner" id="messagesInner"></div>
    </div>

    <div class="composer">
      <div class="composer-inner">
        <button type="button" id="attachBtn" class="icon-btn" title="发送文件">+</button>
        <textarea id="input" rows="1" placeholder="输入消息，Enter 发送，Shift+Enter 换行"></textarea>
        <button type="button" id="sendBtn" class="btn btn-primary send-btn">发送</button>
        <input id="fileInput" type="file" class="hidden">
      </div>
    </div>
  </div>
</div>

<div id="toast" class="toast"></div>
<script src="/assets/user.js" defer></script>
</body>
</html>
"##;

pub const JS: &str = r##"
const $ = (s, r = document) => r.querySelector(s);
const S = {
  base: location.origin,
  token: localStorage.getItem('cw_token') || '',
  me: null, ws: null, retry: false, everConnected: false,
  messages: [], before: null, hasMore: true, loading: false
};

function toast(msg){
  const t = $('#toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(t._t);
  t._t = setTimeout(() => t.classList.remove('show'), 2400);
}
function showErr(sel, msg){ const e = $(sel); e.textContent = msg; e.classList.remove('hidden'); }
function clearErr(sel){ $(sel).classList.add('hidden'); }
function showView(v){ ['auth','change','app'].forEach(n => $('#' + n + 'View').classList.toggle('hidden', n !== v)); }
function esc(s){
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}
function fmtSize(n){
  if(n < 1024) return n + ' B';
  if(n < 1048576) return (n / 1024).toFixed(1) + ' KB';
  return (n / 1048576).toFixed(1) + ' MB';
}
function fmtTime(ts){
  const d = new Date(ts);
  return String(d.getHours()).padStart(2,'0') + ':' + String(d.getMinutes()).padStart(2,'0');
}
function fmtStamp(ts){
  const d = new Date(ts);
  return (d.getMonth()+1) + '/' + d.getDate() + ' ' + fmtTime(ts);
}
function avatarColor(name){
  let h = 0;
  for(const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return 'hsl(' + (h % 360) + ',62%,58%)';
}
function initial(name){ return Array.from(name)[0] || '?'; }

async function api(path, opt = {}){
  const h = Object.assign({}, opt.headers || {});
  if(S.token) h.Authorization = 'Bearer ' + S.token;
  let body = opt.body;
  if(body && !(body instanceof FormData) && typeof body !== 'string'){
    h['Content-Type'] = 'application/json';
    body = JSON.stringify(body);
  }
  const r = await fetch(S.base + path, { method: opt.method || 'GET', headers: h, body: body });
  const ct = r.headers.get('content-type') || '';
  const data = ct.includes('application/json') ? await r.json() : await r.text();
  if(!r.ok) throw new Error((data && data.error) || ('请求失败 ' + r.status));
  return data;
}

function messageHtml(m){
  const me = S.me && m.username === S.me.username;
  let inner;
  if(m.kind === 'file' && m.file){
    inner = '<div class="bubble file-bubble" data-file="' + esc(m.file.id) + '" data-name="' + esc(m.file.name) + '">' +
      '<div class="file-ico">&#9638;</div>' +
      '<div style="min-width:0"><div class="fname">' + esc(m.file.name) + '</div>' +
      '<div class="fsize">' + fmtSize(m.file.size) + ' · ' + esc(m.file.uploader) + ' · 点击下载</div></div>' +
      '<div class="dl">&#8595;</div></div>';
  } else {
    inner = '<div class="bubble">' + esc(m.text) + '</div>';
  }
  return '<div class="msg' + (me ? ' me' : '') + '">' +
    '<div class="avatar" style="background:' + avatarColor(m.username) + '">' + esc(initial(m.username)) + '</div>' +
    '<div class="msg-body"><div class="mname">' + esc(m.username) + ' · ' + fmtStamp(m.ts) + '</div>' +
    inner + '</div></div>';
}

function appendMessage(m, scroll){
  S.messages.push(m);
  $('#messagesInner').insertAdjacentHTML('beforeend', messageHtml(m));
  if(scroll) $('#messages').scrollTop = $('#messages').scrollHeight;
}

async function loadHistory(reset){
  if(S.loading || (!reset && !S.hasMore)) return;
  S.loading = true;
  try{
    let path = '/api/messages?limit=50';
    if(!reset && S.before) path += '&before=' + encodeURIComponent(S.before);
    const d = await api(path);
    const oldHeight = $('#messages').scrollHeight;
    let html = '';
    d.messages.forEach(m => { html += messageHtml(m); });
    S.messages = reset ? d.messages.slice() : d.messages.concat(S.messages);
    if(reset) $('#messagesInner').innerHTML = html;
    else $('#messagesInner').insertAdjacentHTML('afterbegin', html);
    S.hasMore = d.has_more;
    if(S.messages.length) S.before = S.messages[0].key;
    $('#loadMoreBox').classList.toggle('hidden', !S.hasMore);
    if(reset) $('#messages').scrollTop = $('#messages').scrollHeight;
    else $('#messages').scrollTop = $('#messages').scrollHeight - oldHeight;
  }catch(e){ toast(e.message); }
  finally{ S.loading = false; }
}

async function loadFiles(){
  try{
    const files = await api('/api/files');
    $('#fileList').innerHTML = files.length ? files.map(f =>
      '<div class="file-item" data-file="' + esc(f.id) + '" data-name="' + esc(f.name) + '">' +
      '<div class="file-ico">&#9638;</div><div><div class="file-name">' + esc(f.name) + '</div>' +
      '<div class="file-meta">' + fmtSize(f.size) + ' · ' + esc(f.uploader) + '</div></div></div>'
    ).join('') : '<div class="hint">暂无共享文件</div>';
  }catch(e){ toast(e.message); }
}

async function download(id, name){
  try{
    const r = await fetch(S.base + '/api/files/' + encodeURIComponent(id), {
      headers: { Authorization: 'Bearer ' + S.token }
    });
    if(!r.ok) throw new Error('下载失败 ' + r.status);
    const blob = await r.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = name || 'file';
    document.body.appendChild(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 5000);
  }catch(e){ toast(e.message); }
}

function setStatus(ok){
  $('#statusDot').classList.toggle('on', ok);
  $('#statusText').textContent = ok ? '已连接 · 公共大厅' : '连接断开，正在重连';
}

// 重连后补拉最新一页，避免断线期间广播的消息丢失
async function resync(){
  try{
    const d = await api('/api/messages?limit=50');
    const known = new Set(S.messages.map(m => m.id));
    let added = 0;
    d.messages.forEach(m => {
      if(!known.has(m.id)){
        S.messages.push(m);
        $('#messagesInner').insertAdjacentHTML('beforeend', messageHtml(m));
        added++;
      }
    });
    if(added) $('#messages').scrollTop = $('#messages').scrollHeight;
  }catch(e){ /* 补偿失败不影响已有聊天 */ }
}

function connect(){
  if(!S.token) return;
  const u = new URL(S.base + '/api/ws');
  u.protocol = u.protocol === 'https:' ? 'wss:' : 'ws:';
  u.searchParams.set('token', S.token);
  const ws = new WebSocket(u);
  S.ws = ws;
  ws.onopen = () => {
    setStatus(true);
    if(S.everConnected) resync();
    S.everConnected = true;
  };
  ws.onmessage = ev => {
    let d;
    try{ d = JSON.parse(ev.data); }catch(e){ return; }
    if(d.type === 'error'){ toast(d.error); return; }
    if(d.id) appendMessage(d, true);
  };
  ws.onerror = () => ws.close();
  ws.onclose = () => {
    setStatus(false);
    if(S.token && !S.retry){
      S.retry = true;
      setTimeout(() => { S.retry = false; connect(); }, 2500);
    }
  };
}

async function enterApp(){
  S.me = await api('/api/me');
  $('#meName').textContent = S.me.username;
  showView('app');
  await Promise.all([loadHistory(true), loadFiles()]);
  connect();
}

function logout(){
  if(S.ws){ S.ws.onclose = null; S.ws.close(); }
  S.ws = null; S.token = ''; S.me = null;
  localStorage.removeItem('cw_token');
  showView('auth');
}

function send(){
  const t = $('#input');
  const v = t.value.trim();
  if(!v) return;
  if(!S.ws || S.ws.readyState !== 1){ toast('连接中，请稍候'); return; }
  S.ws.send(JSON.stringify({ type: 'text', text: v }));
  t.value = ''; t.style.height = 'auto';
}

async function uploadFile(file){
  const fd = new FormData();
  fd.append('file', file);
  try{
    toast('正在上传 ' + file.name);
    const meta = await api('/api/files', { method: 'POST', body: fd });
    if(!S.ws || S.ws.readyState !== 1) throw new Error('连接未就绪，文件已上传但未发送');
    S.ws.send(JSON.stringify({ type: 'file', file_id: meta.id }));
    await loadFiles();
  }catch(e){ toast(e.message); }
}

function copyAddr(){
  const text = S.base;
  if(navigator.clipboard && navigator.clipboard.writeText){
    navigator.clipboard.writeText(text).then(() => toast('已复制：' + text)).catch(() => toast(text));
  } else { toast(text); }
}

function bind(){
  $('#tabLogin').addEventListener('click', () => {
    $('#tabLogin').classList.add('active'); $('#tabRegister').classList.remove('active');
    $('#loginForm').classList.remove('hidden'); $('#registerForm').classList.add('hidden');
  });
  $('#tabRegister').addEventListener('click', () => {
    $('#tabRegister').classList.add('active'); $('#tabLogin').classList.remove('active');
    $('#registerForm').classList.remove('hidden'); $('#loginForm').classList.add('hidden');
  });

  $('#loginForm').addEventListener('submit', async e => {
    e.preventDefault(); clearErr('#loginErr');
    try{
      const d = await api('/api/login', { method: 'POST', body: {
        username: $('#loginUser').value.trim(), password: $('#loginPass').value
      }});
      S.token = d.token;
      localStorage.setItem('cw_token', d.token);
      if(d.must_change_password) showView('change');
      else await enterApp();
    }catch(err){ showErr('#loginErr', err.message); }
  });

  $('#registerForm').addEventListener('submit', async e => {
    e.preventDefault(); clearErr('#regErr');
    if($('#regPass').value !== $('#regPass2').value){ showErr('#regErr', '两次输入的密码不一致'); return; }
    try{
      await api('/api/register', { method: 'POST', body: {
        username: $('#regUser').value.trim(), password: $('#regPass').value,
        invite_code: $('#regCode').value.trim()
      }});
      toast('注册成功，请登录');
      $('#tabLogin').click();
      $('#loginUser').value = $('#regUser').value.trim();
      $('#loginPass').value = '';
    }catch(err){ showErr('#regErr', err.message); }
  });

  $('#changeForm').addEventListener('submit', async e => {
    e.preventDefault(); clearErr('#changeErr');
    if($('#chgNew').value !== $('#chgNew2').value){ showErr('#changeErr', '两次输入的新密码不一致'); return; }
    try{
      await api('/api/change-password', { method: 'POST', body: {
        old_password: $('#chgOld').value, new_password: $('#chgNew').value
      }});
      toast('密码已修改');
      await enterApp();
    }catch(err){ showErr('#changeErr', err.message); }
  });
  $('#chgLogout').addEventListener('click', logout);

  $('#sendBtn').addEventListener('click', send);
  $('#input').addEventListener('keydown', e => {
    if(e.key === 'Enter' && !e.shiftKey){ e.preventDefault(); send(); }
  });
  $('#input').addEventListener('input', e => {
    e.target.style.height = 'auto';
    e.target.style.height = Math.min(e.target.scrollHeight, 120) + 'px';
  });
  $('#attachBtn').addEventListener('click', () => $('#fileInput').click());
  $('#fileInput').addEventListener('change', e => {
    const f = e.target.files[0];
    if(f) uploadFile(f);
    e.target.value = '';
  });

  $('#loadMore').addEventListener('click', () => loadHistory(false));
  $('#messages').addEventListener('scroll', e => {
    if(e.target.scrollTop < 60) loadHistory(false);
  });
  $('#logoutBtn').addEventListener('click', logout);
  ['#copyAddr','#copyAddr2','#copyAddr3'].forEach(s => $(s).addEventListener('click', copyAddr));

  $('#menuBtn').addEventListener('click', () => {
    $('#sidebar').classList.add('open'); $('#sheetMask').classList.remove('hidden');
  });
  $('#sheetMask').addEventListener('click', () => {
    $('#sidebar').classList.remove('open'); $('#sheetMask').classList.add('hidden');
  });
  document.addEventListener('click', e => {
    const el = e.target.closest('[data-file]');
    if(el){ download(el.dataset.file, el.dataset.name); }
  });
}

(async function init(){
  try{
    const info = await api('/api/info');
    document.title = info.name + ' · 聊天';
    $('#brandName').textContent = info.name;
    $('#chatTitle').textContent = info.name;
    $('#srvAddr').textContent = S.base;
    $('#srvAddr2').textContent = S.base;
    if(!info.allow_registration) $('#tabRegister').classList.add('hidden');
    bind();
    if(S.token){
      try{ await enterApp(); }
      catch(e){ logout(); }
    }
  }catch(e){
    toast('无法连接服务端：' + e.message);
    bind();
  }
})();
"##;
