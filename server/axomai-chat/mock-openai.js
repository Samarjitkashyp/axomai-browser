'use strict';
// A pretend OpenAI for trying the browser's chat without a key: node mock-openai.js  (listens on 127.0.0.1:9100)
// It answers with a short text that shows what it was sent, one word at a time.
const http = require('http');
http.createServer((req, res) => {
  let b = '';
  req.on('data', (c) => (b += c));
  req.on('end', () => {
    let j = {};
    try { j = JSON.parse(b); } catch (_) {}
    const msgs = j.messages || [];
    const last = [...msgs].reverse().find((m) => m.role === 'user');
    const page = msgs.find((m) => m.role === 'system' && /<<<PAGE TEXT>>>/.test(m.content));
    const title = page ? (page.content.match(/Title: (.*)/) || [])[1] : null;
    const answer = `Mock answer. Model: ${j.model}. You asked: "${last ? last.content : ''}". ` + (page ? `I can see the page titled "${title}".` : 'No page was sent.');
    res.writeHead(200, { 'Content-Type': 'text/event-stream' });
    const words = answer.split(' ');
    let i = 0;
    const tick = () => {
      if (i >= words.length) { res.write('data: [DONE]\n\n'); return res.end(); }
      res.write('data: ' + JSON.stringify({ choices: [{ delta: { content: (i ? ' ' : '') + words[i++] } }] }) + '\n\n');
      setTimeout(tick, 60);
    };
    tick();
  });
}).listen(9100, '127.0.0.1');
