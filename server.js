const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 3000;
const UI_DIR = path.join(__dirname, 'ui');

const MIME_TYPES = {
  '.html': 'text/html',
  '.css': 'text/css',
  '.js': 'text/javascript',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
};

const server = http.createServer((req, res) => {
  let reqPath = req.url === '/' ? '/index.html' : req.url;
  let filePath = path.join(UI_DIR, reqPath);
  
  if (!fs.existsSync(filePath)) {
    filePath = path.join(__dirname, reqPath);
  }

  const ext = path.extname(filePath).toLowerCase();
  const contentType = MIME_TYPES[ext] || 'application/octet-stream';

  fs.readFile(filePath, (err, content) => {
    if (err) {
      if (err.code === 'ENOENT') {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end('File Not Found');
      } else {
        res.writeHead(500);
        res.end(`Server Error: ${err.code}`);
      }
    } else {
      res.writeHead(200, { 'Content-Type': contentType });
      res.end(content, 'utf-8');
    }
  });
});

let currentPort = parseInt(process.env.PORT, 10) || 3000;

function startServer(port) {
  server.listen(port, () => {
    console.log(`\n🚀 Axomai Browser UI server running at:`);
    console.log(`   - Main Browser: http://localhost:${port}`);
    console.log(`   - Interactive Demo: http://localhost:${port}/demo.html\n`);
  });
}

server.on('error', (err) => {
  if (err.code === 'EADDRINUSE') {
    console.log(`⚠️ Port ${currentPort} is in use, trying port ${currentPort + 1}...`);
    currentPort += 1;
    setTimeout(() => {
      startServer(currentPort);
    }, 200);
  } else {
    console.error('Server error:', err);
  }
});

startServer(currentPort);
