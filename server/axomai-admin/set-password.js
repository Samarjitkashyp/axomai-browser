// Sets (or changes) the admin password. Run it on the server, as root, so the password is typed only there:
//   sudo node /var/www/axomai-browser/admin/set-password.js
// It stores a salted scrypt hash in /etc/axomai-browser/admin.env (never the password itself) and restarts the service.
'use strict';
const fs = require('fs');
const crypto = require('crypto');
const { execSync } = require('child_process');

const ENV_FILE = process.env.ADMIN_ENV || '/etc/axomai-browser/admin.env';

function ask(prompt) {
  return new Promise((resolve) => {
    process.stdout.write(prompt);
    const stdin = process.stdin;
    let buf = '';
    if (stdin.isTTY) stdin.setRawMode(true);
    stdin.resume();
    stdin.setEncoding('utf8');
    const onData = (ch) => {
      for (const c of ch) {
        if (c === '\r' || c === '\n') { if (stdin.isTTY) stdin.setRawMode(false); stdin.pause(); stdin.removeListener('data', onData); process.stdout.write('\n'); return resolve(buf); }
        if (c === '\u0003') { process.stdout.write('\n'); process.exit(1); }
        if (c === '\u007f' || c === '\b') buf = buf.slice(0, -1); else buf += c;
      }
    };
    stdin.on('data', onData);
  });
}

(async () => {
  const a = await ask('New admin password (at least 14 characters; nothing is shown while typing): ');
  if (a.length < 14) { console.error('Too short. Use at least 14 characters, ideally a few random words.'); process.exit(1); }
  const b = await ask('Type it again: ');
  if (a !== b) { console.error('The two passwords are different.'); process.exit(1); }
  const salt = crypto.randomBytes(16);
  const N = 16384;
  const hash = crypto.scryptSync(a, salt, 64, { N, r: 8, p: 1, maxmem: 128 * 1024 * 1024 });
  let old = '';
  try { old = fs.readFileSync(ENV_FILE, 'utf8'); } catch { /* new file */ }
  const keep = old.split('\n').filter((l) => l && !l.startsWith('ADMIN_PASSWORD_HASH='));
  let secret = (keep.find((l) => l.startsWith('SESSION_SECRET=')) || '').slice('SESSION_SECRET='.length);
  const lines = keep.filter((l) => !l.startsWith('SESSION_SECRET='));
  // a new password also signs everyone out: the session secret changes
  secret = crypto.randomBytes(32).toString('hex');
  lines.push('ADMIN_PASSWORD_HASH=scrypt$' + N + '$' + salt.toString('hex') + '$' + hash.toString('hex'));
  lines.push('SESSION_SECRET=' + secret);
  fs.writeFileSync(ENV_FILE, lines.join('\n') + '\n', { mode: 0o640 });
  try { execSync('chgrp axomai-browser ' + ENV_FILE); } catch { /* not on the server */ }
  try { execSync('systemctl restart axomai-browser-admin'); console.log('Password saved and the admin service restarted.'); } catch { console.log('Password saved. Restart the service: sudo systemctl restart axomai-browser-admin'); }
})();
