// Axomai Browser admin panel: one person edits the landing page text (home in English and Assamese, About, header and footer links).
// No dependencies. Runs on 127.0.0.1 behind Nginx at /admin-browser-axom/.
//
// How it works: the text of the pages lives in site-src (content.py, about.py). This service reads those defaults
// (`build_site.py --dump-content`), keeps the edited text in DATA_DIR/admin/overrides.json, and runs the same page
// generator to preview or publish. A failed build never touches the live site, and every publish keeps a backup.
'use strict';
const http = require('http');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execFile } = require('child_process');

const PORT = Number(process.env.PORT || 8012);
const HOST = process.env.HOST || '127.0.0.1';
const SRC_DIR = process.env.SRC_DIR || '/var/www/axomai-browser/site-src';
const SITE_DIR = process.env.SITE_DIR || '/var/www/axomai-browser/site';
const DOWNLOADS_DIR = process.env.DOWNLOADS_DIR || '/var/www/axomai-browser/downloads';
const DATA_DIR = path.join(process.env.DATA_DIR || '/var/www/axomai-browser/data', 'admin');
const PYTHON = process.env.PYTHON || 'python3';
const HASH = process.env.ADMIN_PASSWORD_HASH || '';
const SECRET = process.env.SESSION_SECRET || '';
const COOKIE_PATH = process.env.COOKIE_PATH || '/admin-browser-axom/';
const COOKIE_SECURE = process.env.COOKIE_SECURE !== '0';
const SESSION_HOURS = 12;
const UI_FILE = path.join(__dirname, 'index.html');

const OVERRIDES = path.join(DATA_DIR, 'overrides.json');
const BACKUPS = path.join(DATA_DIR, 'backups');
const PREVIEW = path.join(DATA_DIR, 'preview');
for (const d of [DATA_DIR, BACKUPS]) fs.mkdirSync(d, { recursive: true });

// ---------------------------------------------------------------- passwords and sessions
function verifyPassword(pw) {
  const parts = HASH.split('$');
  if (parts.length !== 4 || parts[0] !== 'scrypt') return false;
  const [, n, salt, hex] = parts;
  const want = Buffer.from(hex, 'hex');
  const got = crypto.scryptSync(pw, Buffer.from(salt, 'hex'), want.length, { N: Number(n), r: 8, p: 1, maxmem: 128 * 1024 * 1024 });
  return got.length === want.length && crypto.timingSafeEqual(got, want);
}
const sign = (s) => crypto.createHmac('sha256', SECRET).update(s).digest('hex');
function newSession() {
  const exp = Date.now() + SESSION_HOURS * 3600e3;
  const body = exp + '.' + crypto.randomBytes(12).toString('hex');
  return body + '.' + sign(body);
}
function cookies(req) {
  const out = {};
  for (const part of (req.headers.cookie || '').split(';')) {
    const i = part.indexOf('=');
    if (i > 0) out[part.slice(0, i).trim()] = part.slice(i + 1).trim();
  }
  return out;
}
function authed(req) {
  const v = cookies(req).axadmin;
  if (!v || !SECRET) return false;
  const p = v.split('.');
  if (p.length !== 3) return false;
  const good = sign(p[0] + '.' + p[1]);
  const a = Buffer.from(p[2]), b = Buffer.from(good);
  return a.length === b.length && crypto.timingSafeEqual(a, b) && Number(p[0]) > Date.now();
}
const cookieHeader = (value, maxAge) =>
  `axadmin=${value}; HttpOnly; SameSite=Strict; Path=${COOKIE_PATH}; Max-Age=${maxAge}${COOKIE_SECURE ? '; Secure' : ''}`;

// ---------------------------------------------------------------- login limits
const fails = new Map(); // ip -> { n, until }
let globalFails = [];
const WINDOW = 15 * 60e3;
function clientIp(req) {
  return String(req.headers['cf-connecting-ip'] || req.headers['x-real-ip'] || req.socket.remoteAddress || '').split(',')[0].trim();
}
function loginBlocked(ip) {
  const now = Date.now();
  globalFails = globalFails.filter((t) => now - t < WINDOW);
  if (globalFails.length >= 20) return true;
  const f = fails.get(ip);
  return !!(f && f.until && f.until > now);
}
function recordFail(ip) {
  const now = Date.now();
  globalFails.push(now);
  const f = fails.get(ip) || { n: 0, first: now, until: 0 };
  if (now - f.first > WINDOW) { f.n = 0; f.first = now; }
  f.n += 1;
  if (f.n >= 5) f.until = now + WINDOW;
  fails.set(ip, f);
}

// ---------------------------------------------------------------- helpers
function send(res, code, body, type, extra) {
  const buf = Buffer.isBuffer(body) ? body : Buffer.from(typeof body === 'string' ? body : JSON.stringify(body));
  res.writeHead(code, Object.assign({
    'Content-Type': type || 'application/json; charset=utf-8',
    'Cache-Control': 'no-store',
    'X-Content-Type-Options': 'nosniff',
    'X-Robots-Tag': 'noindex, nofollow',
    'Referrer-Policy': 'no-referrer',
  }, extra || {}));
  res.end(buf);
}
const json = (res, code, obj, extra) => send(res, code, obj, 'application/json; charset=utf-8', extra);
function readBody(req, limit = 1.5e6) {
  return new Promise((resolve, reject) => {
    let size = 0; const chunks = [];
    req.on('data', (c) => { size += c.length; if (size > limit) { reject(new Error('Too large')); req.destroy(); } else chunks.push(c); });
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')));
    req.on('error', reject);
  });
}
async function readJson(req) {
  const t = await readBody(req);
  try { return JSON.parse(t || '{}'); } catch { throw new Error('Bad JSON'); }
}
const sameOrigin = (req) => {
  const o = req.headers.origin;
  if (!o) return true;
  try { return new URL(o).host === req.headers.host; } catch { return false; }
};

// ---------------------------------------------------------------- content: defaults, overrides, validation
let defaultsCache = null;
function runPython(args, opts = {}) {
  return new Promise((resolve, reject) => {
    execFile(PYTHON, args, {
      cwd: SRC_DIR, timeout: 60000, maxBuffer: 16e6,
      env: Object.assign({}, process.env, { PYTHONIOENCODING: 'utf-8', PYTHONDONTWRITEBYTECODE: '1' }),
    }, (err, stdout, stderr) => (err ? reject(new Error(String(stderr || err.message).slice(-1500))) : resolve(stdout)));
  });
}
async function defaults() {
  if (!defaultsCache) defaultsCache = JSON.parse(await runPython(['build_site.py', '--dump-content']));
  return defaultsCache;
}
function readOverrides() {
  try { return JSON.parse(fs.readFileSync(OVERRIDES, 'utf8')); } catch { return {}; }
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

// A value must have the same shape as the default: text stays text, rows keep their length, lists hold the same kind of items.
function shapeOk(v, t) {
  if (typeof t === 'string') return typeof v === 'string' && v.length <= 6000;
  if (Array.isArray(t)) {
    if (!Array.isArray(v) || v.length < 1 || v.length > 80) return false;
    const tpl = t[0];
    if (Array.isArray(tpl)) return v.every((row) => Array.isArray(row) && row.length === tpl.length && row.every((x) => typeof x === 'string' && x.length <= 6000));
    return v.every((x) => shapeOk(x, tpl));
  }
  if (t && typeof t === 'object') {
    if (!v || typeof v !== 'object' || Array.isArray(v)) return false;
    const tk = Object.keys(t);
    if (Object.keys(v).length !== tk.length || !tk.every((k) => k in v)) return false;
    return tk.every((k) => shapeOk(v[k], t[k]));
  }
  return false;
}
// Turns the full edited content into "only what differs from the defaults", checking every value on the way.
function toOverrides(content, def) {
  if (!content || typeof content !== 'object') throw new Error('No content');
  const out = {};
  for (const lang of ['en', 'as', 'about']) {
    const given = content[lang] || {};
    for (const [k, v] of Object.entries(given)) {
      if (!(k in def[lang])) throw new Error(`Unknown field: ${lang}.${k}`);
      if (!shapeOk(v, def[lang][k])) throw new Error(`The value of "${k}" (${lang}) has the wrong shape. Reload the page and try again.`);
      if (!same(v, def[lang][k])) (out[lang] = out[lang] || {})[k] = v;
    }
  }
  const s = content.settings || {};
  out.settings = { as_reviewed: !!s.as_reviewed };
  return out;
}
function merged(def, ov) {
  const cur = JSON.parse(JSON.stringify({ en: def.en, as: def.as, about: def.about }));
  for (const lang of ['en', 'as', 'about']) Object.assign(cur[lang], (ov[lang] || {}));
  cur.settings = { as_reviewed: ov.settings ? !!ov.settings.as_reviewed : !!def.settings.as_reviewed };
  return cur;
}

// ---------------------------------------------------------------- builds
function releaseInfo() {
  const info = { version: '0.0.0', size: '28', sha: 'see the .sha256 file next to the installer' };
  try {
    const rel = JSON.parse(fs.readFileSync(path.join(DOWNLOADS_DIR, 'release.json'), 'utf8'));
    info.version = String(rel.tag_name || '').replace(/^v/i, '') || info.version;
    const exe = (rel.assets || []).map((a) => a.name).find((n) => /^Axomai-Setup-.*\.exe$/.test(n));
    if (exe) {
      info.size = String(Math.round(fs.statSync(path.join(DOWNLOADS_DIR, exe)).size / 1048576));
      const h = fs.readFileSync(path.join(DOWNLOADS_DIR, exe + '.sha256'), 'utf8').trim().split(/\s+/)[0].toLowerCase();
      if (/^[0-9a-f]{64}$/.test(h)) info.sha = h;
    }
  } catch { /* keep the fallbacks */ }
  return info;
}
let building = false;
async function build(outDir, overridesFile, preview) {
  if (building) throw new Error('Another build is running. Try again in a few seconds.');
  building = true;
  try {
    const i = releaseInfo();
    const args = ['build_site.py', '--version', i.version, '--size', i.size, '--sha', i.sha, '--out', outDir, '--overrides', overridesFile];
    const android = path.join(DOWNLOADS_DIR, 'android.json');
    if (fs.existsSync(android)) args.push('--android', android);
    if (preview) args.push('--preview');
    await runPython(args);
  } finally { building = false; }
}
function writeAtomic(file, text) {
  const tmp = file + '.tmp' + process.pid;
  fs.writeFileSync(tmp, text, { mode: 0o640 });
  fs.renameSync(tmp, file);
}
const stamp = () => new Date().toISOString().replace(/[:.]/g, '-');

// ---------------------------------------------------------------- routes
const PREVIEW_PAGES = { en: 'index.html', as: 'as/index.html', about: 'about/index.html' };

async function handle(req, res) {
  const url = new URL(req.url, 'http://x');
  const p = url.pathname;
  const post = req.method === 'POST';

  if (req.method === 'GET' && (p === '/' || p === '/index.html')) {
    return send(res, 200, fs.readFileSync(UI_FILE), 'text/html; charset=utf-8', {
      'Content-Security-Policy': "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
      'X-Frame-Options': 'DENY',
    });
  }
  if (p === '/api/status') return json(res, 200, { configured: !!(HASH && SECRET), authed: authed(req) });

  if (post && p === '/api/login') {
    if (!sameOrigin(req) || req.headers['x-axomai-admin'] !== '1') return json(res, 403, { error: 'Forbidden' });
    if (!HASH || !SECRET) return json(res, 503, { error: 'The password has not been set yet (run set-password.js on the server).' });
    const ip = clientIp(req);
    if (loginBlocked(ip)) return json(res, 429, { error: 'Too many wrong passwords. Try again in 15 minutes.' });
    const body = await readJson(req);
    const ok = typeof body.password === 'string' && body.password.length < 300 && verifyPassword(body.password);
    if (!ok) { recordFail(ip); await new Promise((r) => setTimeout(r, 800)); return json(res, 401, { error: 'Wrong password.' }); }
    fails.delete(ip);
    return json(res, 200, { ok: true }, { 'Set-Cookie': cookieHeader(newSession(), SESSION_HOURS * 3600) });
  }
  if (post && p === '/api/logout') return json(res, 200, { ok: true }, { 'Set-Cookie': cookieHeader('', 0) });

  if (!authed(req)) return json(res, 401, { error: 'Please log in.' });
  if (post && (!sameOrigin(req) || req.headers['x-axomai-admin'] !== '1')) return json(res, 403, { error: 'Forbidden' });

  if (req.method === 'GET' && p === '/api/content') {
    const def = await defaults();
    const ov = readOverrides();
    return json(res, 200, { defaults: def, current: merged(def, ov), release: releaseInfo(), backups: listBackups() });
  }
  if (post && p === '/api/preview') {
    const def = await defaults();
    const ov = toOverrides((await readJson(req)).content, def);
    fs.mkdirSync(DATA_DIR, { recursive: true });
    const file = path.join(DATA_DIR, 'preview-overrides.json');
    writeAtomic(file, JSON.stringify(ov));
    fs.rmSync(PREVIEW, { recursive: true, force: true });
    await build(PREVIEW, file, true);
    return json(res, 200, { ok: true });
  }
  if (req.method === 'GET' && p.startsWith('/preview/')) {
    const key = p.slice('/preview/'.length).replace(/\/$/, '');
    const rel = PREVIEW_PAGES[key];
    if (!rel) return json(res, 404, { error: 'Not found' });
    try { return send(res, 200, fs.readFileSync(path.join(PREVIEW, rel)), 'text/html; charset=utf-8'); } catch { return json(res, 404, { error: 'Press Preview first.' }); }
  }
  if (post && p === '/api/publish') {
    const def = await defaults();
    const ov = toOverrides((await readJson(req)).content, def);
    const staging = path.join(DATA_DIR, 'staging-' + stamp());
    const file = staging + '.json';
    writeAtomic(file, JSON.stringify(ov));
    try {
      await build(staging, file, false);
      // the build worked: keep a backup of what was live, then publish
      if (fs.existsSync(OVERRIDES)) fs.copyFileSync(OVERRIDES, path.join(BACKUPS, 'overrides-' + stamp() + '.json'));
      pruneBackups();
      writeAtomic(OVERRIDES, JSON.stringify(ov, null, 1));
      fs.cpSync(staging, SITE_DIR, { recursive: true, force: true });
    } finally {
      fs.rmSync(staging, { recursive: true, force: true });
      fs.rmSync(file, { force: true });
    }
    return json(res, 200, { ok: true, backups: listBackups() });
  }
  if (req.method === 'GET' && p === '/api/backup') {
    const name = url.searchParams.get('name') || '';
    if (!/^overrides-[0-9T-]+Z?\.json$/.test(name)) return json(res, 400, { error: 'Bad name' });
    const def = await defaults();
    let ov;
    try { ov = JSON.parse(fs.readFileSync(path.join(BACKUPS, name), 'utf8')); } catch { return json(res, 404, { error: 'Not found' }); }
    return json(res, 200, { current: merged(def, ov) });
  }
  return json(res, 404, { error: 'Not found' });
}
function listBackups() {
  try { return fs.readdirSync(BACKUPS).filter((n) => /^overrides-.*\.json$/.test(n)).sort().reverse().slice(0, 30); } catch { return []; }
}
function pruneBackups() {
  try { for (const n of fs.readdirSync(BACKUPS).filter((x) => /^overrides-.*\.json$/.test(x)).sort().reverse().slice(60)) fs.rmSync(path.join(BACKUPS, n), { force: true }); } catch { /* ignore */ }
}

http.createServer((req, res) => {
  handle(req, res).catch((e) => {
    if (!res.headersSent) json(res, 400, { error: String(e.message || e).slice(0, 1500) }); else res.end();
  });
}).listen(PORT, HOST, () => console.log(`axomai admin on ${HOST}:${PORT} (password ${HASH ? 'set' : 'NOT set'})`));
