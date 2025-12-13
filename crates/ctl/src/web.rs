//! Embedded web frontend for TTstack management.
//!
//! Serves a single-page application directly from the controller binary.
//! No external files or build tools required.

use axum::response::Html;

/// GET / — serve the management dashboard.
pub async fn index() -> Html<&'static str> {
    Html(FRONTEND_HTML)
}

const FRONTEND_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>TTstack — Dashboard</title>
<style>
  :root {
    --bg: #0f1117; --surface: #1a1d27; --border: #2a2d3a;
    --text: #e4e6ed; --muted: #8b8fa3; --accent: #6c8cff;
    --green: #4ade80; --red: #f87171; --yellow: #fbbf24;
  }
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    background: var(--bg); color: var(--text); line-height: 1.5; }
  .container { max-width: 1200px; margin: 0 auto; padding: 1rem; }

  header { display: flex; align-items: center; justify-content: space-between;
    padding: 1rem 0; border-bottom: 1px solid var(--border); margin-bottom: 1.5rem; }
  header h1 { font-size: 1.4rem; font-weight: 600; }
  header h1 span { color: var(--accent); }

  nav { display: flex; gap: 0.5rem; }
  nav button { background: var(--surface); color: var(--text); border: 1px solid var(--border);
    padding: 0.4rem 1rem; border-radius: 6px; cursor: pointer; font-size: 0.85rem; }
  nav button.active { background: var(--accent); color: #fff; border-color: var(--accent); }
  nav button:hover { border-color: var(--accent); }

  .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 1rem; margin-bottom: 1.5rem; }
  .stat { background: var(--surface); border: 1px solid var(--border);
    border-radius: 8px; padding: 1rem; }
  .stat .label { font-size: 0.75rem; color: var(--muted); text-transform: uppercase;
    letter-spacing: 0.05em; }
  .stat .value { font-size: 1.5rem; font-weight: 700; margin-top: 0.25rem; }

  .panel { background: var(--surface); border: 1px solid var(--border);
    border-radius: 8px; margin-bottom: 1.5rem; }
  .panel-header { display: flex; justify-content: space-between; align-items: center;
    padding: 0.75rem 1rem; border-bottom: 1px solid var(--border); }
  .panel-header h2 { font-size: 1rem; font-weight: 600; }

  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th { text-align: left; padding: 0.6rem 1rem; color: var(--muted);
    font-weight: 500; font-size: 0.75rem; text-transform: uppercase;
    letter-spacing: 0.05em; border-bottom: 1px solid var(--border); }
  td { padding: 0.6rem 1rem; border-bottom: 1px solid var(--border); }
  tr:last-child td { border-bottom: none; }
  tr:hover td { background: rgba(108, 140, 255, 0.05); }

  .badge { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 4px;
    font-size: 0.75rem; font-weight: 500; }
  .badge-online, .badge-active, .badge-running { background: rgba(74,222,128,0.15); color: var(--green); }
  .badge-offline, .badge-stopped { background: rgba(248,113,113,0.15); color: var(--red); }
  .badge-failed { background: rgba(248,113,113,0.25); color: var(--red); }
  .badge-creating { background: rgba(251,191,36,0.15); color: var(--yellow); }

  .btn { background: var(--accent); color: #fff; border: none; padding: 0.35rem 0.8rem;
    border-radius: 5px; cursor: pointer; font-size: 0.8rem; }
  .btn:hover { opacity: 0.85; }
  .btn-danger { background: var(--red); }
  .btn-sm { padding: 0.2rem 0.5rem; font-size: 0.75rem; }

  .modal-overlay { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%;
    background: rgba(0,0,0,0.6); z-index: 100; justify-content: center; align-items: center; }
  .modal-overlay.show { display: flex; }
  .modal { background: var(--surface); border: 1px solid var(--border);
    border-radius: 10px; padding: 1.5rem; width: 90%; max-width: 500px; }
  .modal h3 { margin-bottom: 1rem; }
  .modal label { display: block; font-size: 0.85rem; color: var(--muted); margin-bottom: 0.25rem; }
  .modal input, .modal select { width: 100%; background: var(--bg); color: var(--text);
    border: 1px solid var(--border); padding: 0.5rem; border-radius: 5px;
    margin-bottom: 0.75rem; font-size: 0.85rem; }
  .modal .actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

  .empty { text-align: center; padding: 2rem; color: var(--muted); }
  .toast { position: fixed; bottom: 1rem; right: 1rem; background: var(--surface);
    border: 1px solid var(--border); padding: 0.75rem 1rem; border-radius: 8px;
    z-index: 200; display: none; font-size: 0.85rem; }
  .toast.show { display: block; }
  .toast.error { border-color: var(--red); }
</style>
</head>
<body>
<div class="container">
  <header>
    <h1><span>TT</span>stack Dashboard</h1>
    <nav>
      <button class="active" data-tab="status" onclick="switchTab('status')">Status</button>
      <button data-tab="hosts" onclick="switchTab('hosts')">Hosts</button>
      <button data-tab="envs" onclick="switchTab('envs')">Environments</button>
      <button data-tab="images" onclick="switchTab('images')">Images</button>
    </nav>
  </header>

  <!-- Status Tab -->
  <div id="tab-status" class="tab-content">
    <div class="stats" id="fleet-stats"></div>
  </div>

  <!-- Hosts Tab -->
  <div id="tab-hosts" class="tab-content" style="display:none">
    <div class="panel">
      <div class="panel-header">
        <h2>Hosts</h2>
        <button class="btn" onclick="showModal('add-host')">+ Add Host</button>
      </div>
      <table><thead><tr>
        <th>ID</th><th>Address</th><th>State</th><th>CPU</th><th>Memory</th><th>VMs</th><th></th>
      </tr></thead><tbody id="hosts-body"></tbody></table>
    </div>
  </div>

  <!-- Environments Tab -->
  <div id="tab-envs" class="tab-content" style="display:none">
    <div class="panel">
      <div class="panel-header">
        <h2>Environments</h2>
        <button class="btn" onclick="showModal('add-env')">+ Create Env</button>
      </div>
      <table><thead><tr>
        <th>Name</th><th>Owner</th><th>State</th><th>VMs</th><th></th>
      </tr></thead><tbody id="envs-body"></tbody></table>
    </div>
    <div class="panel" id="env-detail-panel" style="display:none">
      <div class="panel-header"><h2 id="env-detail-title">VM Details</h2></div>
      <table><thead><tr>
        <th>ID</th><th>Image</th><th>Engine</th><th>State</th><th>IP</th><th>Ports</th>
      </tr></thead><tbody id="env-vms-body"></tbody></table>
    </div>
  </div>

  <!-- Images Tab -->
  <div id="tab-images" class="tab-content" style="display:none">
    <div class="panel">
      <div class="panel-header"><h2>Available Images</h2></div>
      <table><thead><tr>
        <th>Name</th><th>Host</th>
      </tr></thead><tbody id="images-body"></tbody></table>
    </div>
  </div>
</div>

<!-- Add Host Modal -->
<div class="modal-overlay" id="modal-add-host">
  <div class="modal">
    <h3>Add Host</h3>
    <label>Agent Address</label>
    <input id="host-addr" placeholder="10.0.0.2:9100">
    <div class="actions">
      <button class="btn" style="background:var(--border)" onclick="hideModals()">Cancel</button>
      <button class="btn" onclick="addHost()">Add</button>
    </div>
  </div>
</div>

<!-- Create Environment Modal -->
<div class="modal-overlay" id="modal-add-env">
  <div class="modal">
    <h3>Create Environment</h3>
    <label>Name</label>
    <input id="env-name" placeholder="my-test-env">
    <label>Image</label>
    <input id="env-image" placeholder="ubuntu-22.04">
    <label>Engine</label>
    <select id="env-engine">
      <option value="qemu">QEMU</option>
      <option value="firecracker">Firecracker</option>
      <option value="docker">Docker</option>
      <option value="bhyve">Bhyve</option>
    </select>
    <label>CPU Cores</label>
    <input id="env-cpu" type="number" value="2">
    <label>Memory (MB)</label>
    <input id="env-mem" type="number" value="1024">
    <label>Replicas</label>
    <input id="env-dup" type="number" value="1">
    <div class="actions">
      <button class="btn" style="background:var(--border)" onclick="hideModals()">Cancel</button>
      <button class="btn" onclick="createEnv()">Create</button>
    </div>
  </div>
</div>

<div class="toast" id="toast"></div>

<script>
const API = '';

async function api(method, path, body) {
  const opts = { method, headers: { 'Content-Type': 'application/json' } };
  if (body) opts.body = JSON.stringify(body);
  const res = await fetch(API + path, opts);
  const data = await res.json();
  if (!data.ok) throw new Error(data.error || 'Request failed');
  return data.data;
}

function switchTab(name) {
  document.querySelectorAll('.tab-content').forEach(el => el.style.display = 'none');
  document.querySelectorAll('nav button').forEach(el => el.classList.remove('active'));
  document.getElementById('tab-' + name).style.display = 'block';
  document.querySelector(`nav button[data-tab="${name}"]`).classList.add('active');
  refresh(name);
}

function badge(state) {
  return `<span class="badge badge-${state}">${state}</span>`;
}

function toast(msg, isError) {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.className = 'toast show' + (isError ? ' error' : '');
  setTimeout(() => el.className = 'toast', 3000);
}

function showModal(id) { document.getElementById('modal-' + id).classList.add('show'); }
function hideModals() { document.querySelectorAll('.modal-overlay').forEach(el => el.classList.remove('show')); }

async function refresh(tab) {
  try {
    if (tab === 'status') await loadStatus();
    else if (tab === 'hosts') await loadHosts();
    else if (tab === 'envs') await loadEnvs();
    else if (tab === 'images') await loadImages();
  } catch (e) { toast(e.message, true); }
}

async function loadStatus() {
  const s = await api('GET', '/api/status');
  document.getElementById('fleet-stats').innerHTML = `
    <div class="stat"><div class="label">Hosts</div><div class="value">${s.hosts_online}/${s.hosts}</div></div>
    <div class="stat"><div class="label">VMs</div><div class="value">${s.total_vms}</div></div>
    <div class="stat"><div class="label">Envs</div><div class="value">${s.total_envs}</div></div>
    <div class="stat"><div class="label">CPU</div><div class="value">${s.cpu_used}/${s.cpu_total}</div></div>
    <div class="stat"><div class="label">Memory</div><div class="value">${s.mem_used}/${s.mem_total} MB</div></div>
    <div class="stat"><div class="label">Disk</div><div class="value">${s.disk_used}/${s.disk_total} MB</div></div>
  `;
}

async function loadHosts() {
  const hosts = await api('GET', '/api/hosts');
  const tbody = document.getElementById('hosts-body');
  if (!hosts.length) { tbody.innerHTML = '<tr><td colspan="7" class="empty">No hosts registered</td></tr>'; return; }
  tbody.innerHTML = hosts.map(h => `<tr>
    <td>${h.id}</td><td>${h.addr}</td><td>${badge(h.state)}</td>
    <td>${h.resource.cpu_used}/${h.resource.cpu_total}</td>
    <td>${h.resource.mem_used}/${h.resource.mem_total} MB</td>
    <td>${h.resource.vm_count}</td>
    <td><button class="btn btn-sm btn-danger" onclick="removeHost('${h.id}')">Remove</button></td>
  </tr>`).join('');
}

async function loadEnvs() {
  const envs = await api('GET', '/api/envs');
  const tbody = document.getElementById('envs-body');
  document.getElementById('env-detail-panel').style.display = 'none';
  if (!envs.length) { tbody.innerHTML = '<tr><td colspan="5" class="empty">No environments</td></tr>'; return; }
  tbody.innerHTML = envs.map(e => `<tr>
    <td><a href="#" onclick="showEnv('${e.id}'); return false;" style="color:var(--accent)">${e.id}</a></td>
    <td>${e.owner}</td><td>${badge(e.state)}</td><td>${e.vm_ids.length}</td>
    <td>
      <button class="btn btn-sm" onclick="toggleEnv('${e.id}','${e.state}')">${e.state==='active'?'Stop':'Start'}</button>
      <button class="btn btn-sm btn-danger" onclick="deleteEnv('${e.id}')">Delete</button>
    </td>
  </tr>`).join('');
}

async function showEnv(id) {
  try {
    const detail = await api('GET', '/api/envs/' + id);
    document.getElementById('env-detail-title').textContent = 'VMs in ' + id;
    document.getElementById('env-detail-panel').style.display = 'block';
    const tbody = document.getElementById('env-vms-body');
    tbody.innerHTML = detail.vms.map(vm => {
      const ports = Object.entries(vm.port_map).map(([g,h]) => h+'→'+g).join(', ');
      return `<tr>
        <td>${vm.id}</td><td>${vm.image}</td><td>${vm.engine}</td>
        <td>${badge(vm.state)}</td><td>${vm.ip}</td><td>${ports||'-'}</td>
      </tr>`;
    }).join('');
  } catch (e) { toast(e.message, true); }
}

async function loadImages() {
  const images = await api('GET', '/api/images');
  const tbody = document.getElementById('images-body');
  if (!images.length) { tbody.innerHTML = '<tr><td colspan="2" class="empty">No images available</td></tr>'; return; }
  tbody.innerHTML = images.map(i => `<tr><td>${i.name}</td><td>${i.host_id}</td></tr>`).join('');
}

async function addHost() {
  const addr = document.getElementById('host-addr').value.trim();
  if (!addr) return;
  try {
    await api('POST', '/api/hosts', { addr });
    hideModals(); toast('Host added'); loadHosts();
  } catch (e) { toast(e.message, true); }
}

async function removeHost(id) {
  if (!confirm('Remove host ' + id + '?')) return;
  try { await api('DELETE', '/api/hosts/' + id); toast('Host removed'); loadHosts(); }
  catch (e) { toast(e.message, true); }
}

async function createEnv() {
  const name = document.getElementById('env-name').value.trim();
  const image = document.getElementById('env-image').value.trim();
  const engine = document.getElementById('env-engine').value;
  const cpu = parseInt(document.getElementById('env-cpu').value) || 2;
  const mem = parseInt(document.getElementById('env-mem').value) || 1024;
  const dup = parseInt(document.getElementById('env-dup').value) || 1;
  if (!name || !image) return;

  const vms = [];
  for (let i = 0; i < dup; i++) vms.push({ image, engine, cpu, mem, ports: [22], deny_outgoing: false });

  try {
    await api('POST', '/api/envs', { id: name, owner: 'web', vms, lifetime: null });
    hideModals(); toast('Environment created'); loadEnvs();
  } catch (e) { toast(e.message, true); }
}

async function deleteEnv(id) {
  if (!confirm('Delete environment ' + id + '?')) return;
  try { await api('DELETE', '/api/envs/' + id); toast('Environment deleted'); loadEnvs(); }
  catch (e) { toast(e.message, true); }
}

async function toggleEnv(id, state) {
  const action = state === 'active' ? 'stop' : 'start';
  try { await api('POST', '/api/envs/' + id + '/' + action, {}); toast('Environment ' + action + 'ed'); loadEnvs(); }
  catch (e) { toast(e.message, true); }
}

// Initial load
loadStatus();
</script>
</body>
</html>
"##;
