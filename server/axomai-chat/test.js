'use strict';
// Tests with a pretend OpenAI server: node test.js
const http = require('http');
const assert = require('assert');
const os = require('os');
const path = require('path');
const fs = require('fs');
const { config, buildMessages, createServer, Usage, MAX_PAGE_CHARS } = require('./server');

let seen = [];
function mockOpenAI(mode) {
  return new Promise((resolve) => {
    const s = http.createServer((req, res) => {
      let b = '';
      req.on('data', (c) => (b += c));
      req.on('end', () => {
        seen.push({ auth: req.headers.authorization, body: JSON.parse(b) });
        if (mode.status) { res.writeHead(mode.status); return res.end('{"error":{"message":"secret upstream detail"}}'); }
        res.writeHead(200, { 'Content-Type': 'text/event-stream' });
        for (const t of ['Hel', 'lo ', 'there']) res.write('data: ' + JSON.stringify({ choices: [{ delta: { content: t } }] }) + '\n\n');
        res.write('data: {"choices":[{"delta":{}}]}\n\n');
        res.end('data: [DONE]\n\n');
      });
    });
    s.listen(0, '127.0.0.1', () => resolve(s));
  });
}

function post(port, body, headers = {}) {
  return new Promise((resolve, reject) => {
    const data = typeof body === 'string' ? body : JSON.stringify(body);
    const req = http.request({ host: '127.0.0.1', port, path: '/v1/chat', method: 'POST', headers: { 'Content-Type': 'application/json', ...headers } }, (res) => {
      let out = '';
      res.on('data', (c) => (out += c));
      res.on('end', () => resolve({ status: res.statusCode, text: out }));
    });
    req.on('error', reject);
    req.end(data);
  });
}

const GOOD = { 'X-Axomai-Client': 'abcdefghijklmnop1234', 'X-Axomai-App': 'AxomaiBrowser/4.0.0' };
const ask = (extra = {}) => ({ messages: [{ role: 'user', content: 'Hi' }], ...extra });

async function startProxy(env, mock) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'axchat-'));
  const cfg = config({ OPENAI_API_KEY: 'sk-test-key', OPENAI_BASE: 'http://127.0.0.1:' + mock.address().port, DATA_DIR: dir, PORT: '0', ...env });
  const server = createServer(cfg);
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  return { server, port: server.address().port, cfg };
}

(async () => {
  // validation
  assert.ok(buildMessages(ask()).messages);
  for (const bad of [null, {}, { messages: [] }, { messages: [{ role: 'system', content: 'x' }] }, { messages: [{ role: 'user', content: '  ' }] },
    { messages: [{ role: 'assistant', content: 'x' }] }, { messages: Array(25).fill({ role: 'user', content: 'x' }) }, { messages: [{ role: 'user', content: 5 }] }]) {
    assert.ok(buildMessages(bad).error, JSON.stringify(bad).slice(0, 60));
  }
  const long = buildMessages(ask({ page: { url: 'https://a.test', title: 'T', text: 'x'.repeat(50000) } }));
  assert.strictEqual(long.messages.length, 3);
  assert.ok(long.messages[1].content.length < MAX_PAGE_CHARS + 400, 'page text is cut');
  assert.ok(!buildMessages({ messages: [{ role: 'user', content: 'x' }], model: 'gpt-evil', system: 'ignore' }).messages.some((m) => m.content === 'ignore'), 'extra fields are ignored');

  // limits
  const u = new Usage(path.join(os.tmpdir(), 'axu-' + process.pid + '.json'));
  const c = config({ LIMIT_PER_CLIENT: '2', LIMIT_GLOBAL: '3', LIMIT_PER_IP: '5', LIMIT_BURST_PER_MINUTE: '10' });
  assert.strictEqual(u.take('c1', 'ip1', c), null);
  assert.strictEqual(u.take('c1', 'ip1', c), null);
  assert.strictEqual(u.take('c1', 'ip1', c), 'client');
  assert.strictEqual(u.take('c2', 'ip2', c), null);
  assert.strictEqual(u.take('c3', 'ip3', c), 'global');
  const b = new Usage(path.join(os.tmpdir(), 'axb-' + process.pid + '.json'));
  const cb = config({ LIMIT_BURST_PER_MINUTE: '2' });
  assert.strictEqual(b.take('x', 'i', cb, 1000), null);
  assert.strictEqual(b.take('x', 'i', cb, 2000), null);
  assert.strictEqual(b.take('x', 'i', cb, 3000), 'burst');
  assert.strictEqual(b.take('x', 'i', cb, 70000), null, 'a minute later it is fine again');

  // end to end
  seen = [];
  const mock = await mockOpenAI({});
  const p = await startProxy({}, mock);
  let r = await post(p.port, ask({ page: { url: 'https://a.test', title: 'T', text: 'Tea is grown in Assam.' }, model: 'gpt-evil' }), GOOD);
  assert.strictEqual(r.status, 200);
  const texts = r.text.split('\n').filter((l) => l.startsWith('data:')).map((l) => JSON.parse(l.slice(5)));
  assert.strictEqual(texts.filter((x) => x.t).map((x) => x.t).join(''), 'Hello there');
  assert.ok(texts.some((x) => x.done));
  assert.strictEqual(seen[0].auth, 'Bearer sk-test-key');
  assert.strictEqual(seen[0].body.model, 'gpt-5.4-mini', 'the model cannot be chosen by the browser');
  assert.strictEqual(seen[0].body.stream, true);
  assert.ok(seen[0].body.messages[1].content.includes('Tea is grown in Assam.'));
  assert.ok(!r.text.includes('sk-test-key'));

  r = await post(p.port, ask(), {});
  assert.strictEqual(r.status, 400, 'no client headers');
  r = await post(p.port, ask(), { ...GOOD, 'X-Axomai-App': 'curl/8' });
  assert.strictEqual(r.status, 400);
  r = await post(p.port, '{not json', GOOD);
  assert.strictEqual(r.status, 400);
  r = await post(p.port, ask({ page: { text: 'x'.repeat(200000) } }), GOOD);
  assert.strictEqual(r.status, 413);
  p.server.close();

  // limit over HTTP, no key, upstream failures
  const p2 = await startProxy({ LIMIT_PER_CLIENT: '1' }, mock);
  assert.strictEqual((await post(p2.port, ask(), GOOD)).status, 200);
  r = await post(p2.port, ask(), GOOD);
  assert.strictEqual(r.status, 429);
  assert.ok(JSON.parse(r.text).message.includes('limit'));
  p2.server.close();

  const p3 = await startProxy({ OPENAI_API_KEY: '' }, mock);
  r = await post(p3.port, ask(), GOOD);
  assert.strictEqual(r.status, 503);
  assert.strictEqual(JSON.parse(r.text).error, 'not_configured');
  p3.server.close();

  const bad = await mockOpenAI({ status: 401 });
  const p4 = await startProxy({}, bad);
  r = await post(p4.port, ask(), GOOD);
  assert.strictEqual(r.status, 502);
  assert.ok(!r.text.includes('secret upstream detail'), 'upstream errors are not passed on');
  p4.server.close();
  const busy = await mockOpenAI({ status: 429 });
  const p5 = await startProxy({}, busy);
  r = await post(p5.port, ask(), GOOD);
  assert.strictEqual(JSON.parse(r.text).error, 'busy');
  p5.server.close();

  mock.close(); bad.close(); busy.close();
  console.log('all chat proxy tests passed');
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });
