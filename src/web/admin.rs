/// /admin 管理面板：HTML、样式引用与全部交互逻辑都由本 .rs 文件生成。
/// 管理员是服务器级身份：登录只需管理密码（由 passwd 子命令设定），没有用户名。
pub const PAGE: &str = r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>Chatware · 管理面板</title>
<link rel="stylesheet" href="/assets/style.css">
</head>
<body>

<div id="loginView" class="auth-wrap">
  <div class="auth-card">
    <div class="brand"><div class="brand-mark">C</div><h1>Chatware 管理</h1></div>
    <p class="brand-sub">服务器管理面板 · 凭管理密码登录</p>
    <div id="loginErr" class="err hidden"></div>
    <form id="loginForm">
      <div class="field"><label>管理密码</label><input class="input" id="loginPass" type="password" autocomplete="current-password" required></div>
      <button class="btn btn-primary btn-block" type="submit">登录</button>
    </form>
    <p class="hint">初始管理密码只在服务端启动日志中打印一次；可用 ./chatware passwd -d 数据目录 --password 新密码 自行设定</p>
    <p class="hint"><a href="/user">前往 /user 聊天面板</a></p>
  </div>
</div>

<div id="adminView" class="hidden">
  <div class="topbar">
    <div class="brand-mark">C</div>
    <div class="title">Chatware 管理面板</div>
    <div class="spacer"></div>
    <a href="/user" class="btn btn-ghost btn-sm">/user</a>
    <span class="tag">服务器管理员</span>
    <button type="button" id="logoutBtn" class="btn btn-ghost btn-sm">退出</button>
  </div>

  <div class="wrap">
    <h1 class="page-title">概览</h1>
    <p class="page-sub">管理注册账号、注册邀请码与服务端运行参数</p>
    <div class="navtabs">
      <button type="button" data-tab="overview" class="active">概览</button>
      <button type="button" data-tab="invites">邀请码</button>
      <button type="button" data-tab="users">账号</button>
      <button type="button" data-tab="settings">设置</button>
      <button type="button" data-tab="conn">客户端连接</button>
    </div>

    <div class="pane" id="pane-overview">
      <div class="grid">
        <div class="stat"><div class="label">注册用户</div><div class="value" id="statUsers">-</div></div>
        <div class="stat"><div class="label">聊天消息</div><div class="value" id="statMessages">-</div></div>
        <div class="stat"><div class="label">共享文件</div><div class="value" id="statFiles">-</div></div>
        <div class="stat"><div class="label">邀请码</div><div class="value" id="statInvites">-</div></div>
      </div>
      <div class="card">
        <h3>运行信息</h3>
        <div class="row" style="gap:24px">
          <div><div class="label" style="color:var(--muted);font-size:12px">文件占用</div><div id="statSize" class="mono">-</div></div>
          <div><div class="label" style="color:var(--muted);font-size:12px">启动时间</div><div id="statStarted" class="mono">-</div></div>
        </div>
        <p class="hint" style="text-align:left">服务端只监听 HTTP，TLS 域名证书请由 Caddy / Nginx 等反向代理提供。</p>
      </div>
    </div>

    <div class="pane hidden" id="pane-invites">
      <div class="card">
        <h3>生成注册邀请码</h3>
        <div class="row">
          <input class="input" id="invMax" type="number" min="1" max="1000" value="1" style="max-width:130px" title="可用次数">
          <input class="input" id="invHours" type="number" min="0" value="168" style="max-width:150px" title="有效小时，0 表示永不过期">
          <input class="input" id="invNote" placeholder="备注（可选）">
          <button type="button" id="invCreate" class="btn btn-primary">生成</button>
        </div>
        <p class="hint" style="text-align:left">依次为：可用次数、有效小时（0 = 永不过期）、备注</p>
      </div>
      <div class="card">
        <h3>邀请码列表</h3>
        <div class="table-scroll"><table>
          <thead><tr><th>邀请码</th><th>使用情况</th><th>有效期</th><th>备注</th><th>操作</th></tr></thead>
          <tbody id="invBody"></tbody>
        </table></div>
      </div>
    </div>

    <div class="pane hidden" id="pane-users">
      <div class="card">
        <h3>注册账号</h3>
        <div class="table-scroll"><table>
          <thead><tr><th>用户名</th><th>状态</th><th>注册时间</th><th>操作</th></tr></thead>
          <tbody id="userBody"></tbody>
        </table></div>
      </div>
    </div>

    <div class="pane hidden" id="pane-settings">
      <div class="card">
        <h3>运行参数</h3>
        <div class="field"><label class="switch"><input type="checkbox" id="cfgReg"> 开放注册（关闭后只能由已有账号登录）</label></div>
        <div class="field" style="max-width:260px"><label>单文件大小上限（MB）</label><input class="input" id="cfgSize" type="number" min="1" value="100"></div>
        <div class="field" style="max-width:420px"><label>服务端名称（显示在 /user 面板标题）</label><input class="input" id="cfgName" maxlength="64" placeholder="Chatware"></div>
        <div class="field" style="max-width:420px"><label>对外地址（反向代理后的域名，供客户端连接页使用）</label><input class="input" id="cfgUrl" placeholder="https://chat.example.com"></div>
        <div class="field" style="max-width:260px"><label>单条消息长度上限（字符）</label><input class="input" id="cfgMsg" type="number" min="1" value="4000"></div>
        <button type="button" id="cfgSave" class="btn btn-primary">保存设置</button>
        <p class="hint" style="text-align:left">修改立即生效，并写入数据库（重启后保留）。对外地址留空时，客户端连接页回退为 http://监听地址。</p>
      </div>
      <div class="card">
        <h3>只读信息（由启动参数决定，需重启生效）</h3>
        <div id="cfgReadonly" class="mono" style="line-height:2;word-break:break-all"></div>
      </div>
      <div class="card">
        <h3>修改管理密码</h3>
        <div id="pwErr" class="err hidden"></div>
        <form id="pwForm">
          <div class="field" style="max-width:320px"><label>当前管理密码</label><input class="input" id="pwCurrent" type="password" autocomplete="current-password" required></div>
          <div class="field" style="max-width:320px"><label>新管理密码（8~128 个字符）</label><input class="input" id="pwNew" type="password" autocomplete="new-password" required></div>
          <div class="field" style="max-width:320px"><label>确认新管理密码</label><input class="input" id="pwNew2" type="password" autocomplete="new-password" required></div>
          <button type="submit" class="btn btn-primary">修改密码</button>
        </form>
        <p class="hint" style="text-align:left">修改后所有管理会话立即失效，需用新密码重新登录；不影响任何聊天用户账号。忘记密码时可离线用 ./chatware passwd 设定。</p>
      </div>
    </div>

    <div class="pane hidden" id="pane-conn">
      <div class="card">
        <h3>客户端连接信息</h3>
        <p class="hint" style="text-align:left;margin-top:0">把下面地址填到安卓客户端登录 / 注册页的「服务端地址」中。</p>
        <div class="row"><div id="connUrl" class="mono" style="flex:1;word-break:break-all"></div><button type="button" id="connCopy" class="btn btn-ghost btn-sm">复制</button></div>
        <p class="hint" style="text-align:left">域名证书由反向代理提供，客户端走系统默认 CA 校验，无需填写证书指纹，也没有跳过校验开关。</p>
      </div>
    </div>
  </div>
</div>

<div id="toast" class="toast"></div>
<script src="/assets/admin.js" defer></script>
</body>
</html>
"##;

pub const JS: &str = r##"
const $ = (s, r = document) => r.querySelector(s);
const S = { base: location.origin, token: localStorage.getItem('cw_admin_token') || '', tab: 'overview' };

function toast(msg){
  const t = $('#toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(t._t);
  t._t = setTimeout(() => t.classList.remove('show'), 2400);
}
function showErr(sel, msg){ const e = $(sel); e.textContent = msg; e.classList.remove('hidden'); }
function clearErr(sel){ $(sel).classList.add('hidden'); }
function showView(v){ ['login','admin'].forEach(n => $('#' + n + 'View').classList.toggle('hidden', n !== v)); }
function esc(s){
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}
function fmtSize(n){
  if(n < 1024) return n + ' B';
  if(n < 1048576) return (n / 1024).toFixed(1) + ' KB';
  return (n / 1048576).toFixed(1) + ' MB';
}
function fmtDate(ts){
  if(!ts) return '永不';
  const d = new Date(ts);
  return d.getFullYear() + '-' + String(d.getMonth()+1).padStart(2,'0') + '-' + String(d.getDate()).padStart(2,'0') + ' ' + String(d.getHours()).padStart(2,'0') + ':' + String(d.getMinutes()).padStart(2,'0');
}

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

function copyText(text){
  if(navigator.clipboard && navigator.clipboard.writeText){
    navigator.clipboard.writeText(text).then(() => toast('已复制')).catch(() => toast(text));
  } else { toast(text); }
}

async function loadSummary(){
  try{
    const d = await api('/api/admin/summary');
    $('#statUsers').textContent = d.users;
    $('#statMessages').textContent = d.messages;
    $('#statFiles').textContent = d.files;
    $('#statInvites').textContent = d.invites;
    $('#statSize').textContent = fmtSize(d.total_size);
    $('#statStarted').textContent = fmtDate(d.started_at);
  }catch(e){ toast(e.message); }
}

async function loadInvites(){
  try{
    const list = await api('/api/admin/invites');
    const now = Date.now();
    $('#invBody').innerHTML = list.length ? list.map(v => {
      const expired = v.expires_at !== 0 && v.expires_at <= now;
      const usedUp = v.used >= v.max_uses;
      const state = expired ? '<span class="tag gray">已过期</span>' : usedUp ? '<span class="tag red">已用尽</span>' : '<span class="tag green">可用</span>';
      return '<tr><td class="mono">' + esc(v.code) + '</td>' +
        '<td>' + v.used + ' / ' + v.max_uses + ' ' + state + '</td>' +
        '<td>' + fmtDate(v.expires_at) + '</td>' +
        '<td>' + esc(v.note || '-') + '</td>' +
        '<td><button type="button" class="btn btn-ghost btn-sm" data-copy="' + esc(v.code) + '">复制</button> ' +
        '<button type="button" class="btn btn-danger btn-sm" data-del="' + esc(v.code) + '">删除</button></td></tr>';
    }).join('') : '<tr><td colspan="5" class="hint">暂无邀请码</td></tr>';
  }catch(e){ toast(e.message); }
}

async function loadUsers(){
  try{
    const list = await api('/api/admin/users');
    $('#userBody').innerHTML = list.map(u =>
      '<tr><td><b>' + esc(u.username) + '</b></td>' +
      '<td>' + (u.disabled ? '<span class="tag red">已禁用</span>' : '<span class="tag green">正常</span>') + '</td>' +
      '<td>' + fmtDate(u.created_at) + '</td>' +
      '<td><button type="button" class="btn btn-ghost btn-sm" data-toggle="' + esc(u.username) + '" data-disabled="' + (u.disabled ? '1' : '0') + '">' + (u.disabled ? '启用' : '禁用') + '</button> ' +
      '<button type="button" class="btn btn-danger btn-sm" data-remove="' + esc(u.username) + '">删除</button></td></tr>'
    ).join('');
  }catch(e){ toast(e.message); }
}

async function loadConfig(){
  try{
    const d = await api('/api/admin/config');
    $('#cfgReg').checked = d.allow_registration;
    $('#cfgSize').value = d.max_file_size_mb;
    $('#cfgName').value = d.server_name;
    $('#cfgUrl').value = d.public_url;
    $('#cfgMsg').value = d.max_message_length;
    $('#cfgReadonly').innerHTML =
      '监听地址：' + esc(d.bind) + '<br>' +
      '数据目录：' + esc(d.data_dir);
  }catch(e){ toast(e.message); }
}

async function loadConn(){
  try{
    const d = await api('/api/admin/connection-info');
    $('#connUrl').textContent = d.public_url;
  }catch(e){ toast(e.message); }
}

function switchTab(tab){
  S.tab = tab;
  document.querySelectorAll('.navtabs button').forEach(b => b.classList.toggle('active', b.dataset.tab === tab));
  document.querySelectorAll('.pane').forEach(p => p.classList.toggle('hidden', p.id !== 'pane-' + tab));
  const loaders = { overview: loadSummary, invites: loadInvites, users: loadUsers, settings: loadConfig, conn: loadConn };
  if(loaders[tab]) loaders[tab]();
}

async function enterAdmin(){
  await api('/api/admin/me');
  showView('admin');
  switchTab('overview');
}

async function logout(){
  try{ if(S.token) await api('/api/admin/logout', { method: 'POST' }); }catch(e){ /* 本地登出照常进行 */ }
  S.token = '';
  localStorage.removeItem('cw_admin_token');
  showView('login');
}

function bind(){
  $('#loginForm').addEventListener('submit', async e => {
    e.preventDefault(); clearErr('#loginErr');
    try{
      const d = await api('/api/admin/login', { method: 'POST', body: { password: $('#loginPass').value }});
      S.token = d.token;
      localStorage.setItem('cw_admin_token', d.token);
      $('#loginPass').value = '';
      await enterAdmin();
    }catch(err){ showErr('#loginErr', err.message); }
  });

  $('#logoutBtn').addEventListener('click', logout);

  document.querySelectorAll('.navtabs button').forEach(b => b.addEventListener('click', () => switchTab(b.dataset.tab)));

  $('#invCreate').addEventListener('click', async () => {
    try{
      await api('/api/admin/invites', { method: 'POST', body: {
        max_uses: Number($('#invMax').value) || 1,
        expires_hours: Number($('#invHours').value) || 0,
        note: $('#invNote').value.trim()
      }});
      $('#invNote').value = '';
      toast('邀请码已生成');
      loadInvites();
    }catch(e){ toast(e.message); }
  });

  $('#invBody').addEventListener('click', async e => {
    const b = e.target.closest('button');
    if(!b) return;
    if(b.dataset.copy){ copyText(b.dataset.copy); return; }
    if(b.dataset.del){
      if(!confirm('确认删除邀请码 ' + b.dataset.del + '？')) return;
      try{ await api('/api/admin/invites/' + encodeURIComponent(b.dataset.del), { method: 'DELETE' }); loadInvites(); }
      catch(err){ toast(err.message); }
    }
  });

  $('#userBody').addEventListener('click', async e => {
    const b = e.target.closest('button');
    if(!b) return;
    try{
      if(b.dataset.toggle){
        const action = b.dataset.disabled === '1' ? 'enable' : 'disable';
        await api('/api/admin/users/' + encodeURIComponent(b.dataset.toggle) + '/' + action, { method: 'POST' });
        loadUsers();
      } else if(b.dataset.remove){
        if(!confirm('确认删除账号 ' + b.dataset.remove + '？其历史消息与文件会保留。')) return;
        await api('/api/admin/users/' + encodeURIComponent(b.dataset.remove), { method: 'DELETE' });
        loadUsers();
      }
    }catch(err){ toast(err.message); }
  });

  $('#cfgSave').addEventListener('click', async () => {
    const size = Number($('#cfgSize').value);
    const msg = Number($('#cfgMsg').value);
    if(!Number.isInteger(size) || size < 1){ toast('文件大小上限需为不小于 1 的整数'); return; }
    if(!Number.isInteger(msg) || msg < 1){ toast('消息长度上限需为不小于 1 的整数'); return; }
    try{
      await api('/api/admin/config', { method: 'POST', body: {
        allow_registration: $('#cfgReg').checked,
        max_file_size_mb: size,
        server_name: $('#cfgName').value,
        public_url: $('#cfgUrl').value,
        max_message_length: msg
      }});
      toast('设置已保存并生效');
      loadConfig();
    }catch(e){ toast(e.message); }
  });

  $('#pwForm').addEventListener('submit', async e => {
    e.preventDefault(); clearErr('#pwErr');
    const np = $('#pwNew').value;
    if(np !== $('#pwNew2').value){ showErr('#pwErr', '两次输入的新密码不一致'); return; }
    try{
      await api('/api/admin/password', { method: 'POST', body: {
        current_password: $('#pwCurrent').value, new_password: np
      }});
      S.token = '';
      localStorage.removeItem('cw_admin_token');
      $('#pwCurrent').value = ''; $('#pwNew').value = ''; $('#pwNew2').value = '';
      showView('login');
      toast('管理密码已修改，请用新密码重新登录');
    }catch(err){ showErr('#pwErr', err.message); }
  });

  $('#connCopy').addEventListener('click', () => copyText($('#connUrl').textContent));
}

(async function init(){
  bind();
  if(S.token){
    try{ await enterAdmin(); }
    catch(e){ logout(); }
  }
})();
"##;
