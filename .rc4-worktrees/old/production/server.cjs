#!/usr/bin/env node
/**
 * X3 Production Server
 * Serves x3star.net static site with production-grade configuration
 */

const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 8080;
const PUBLIC_DIR = path.join(__dirname, 'public');

// MIME types for custom extensions
const MIME_TYPES = {
  '.js': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.ico': 'image/x-icon',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
};

const server = http.createServer((req, res) => {
  // Normalize URL
  let pathname = decodeURIComponent(req.url);
  if (pathname === '/') pathname = '/x3star-index.html';
  if (!pathname.includes('.')) pathname += '.html';

  // Resolve file path
  const filepath = path.join(PUBLIC_DIR, pathname);

  // Security: prevent directory traversal
  const realpath = path.resolve(filepath);
  if (!realpath.startsWith(PUBLIC_DIR)) {
    res.writeHead(403, { 'Content-Type': 'text/plain' });
    res.end('403 Forbidden');
    return;
  }

  // Read and serve file
  fs.readFile(filepath, (err, content) => {
    if (err) {
      // Return 404 for missing files
      res.writeHead(404, { 'Content-Type': 'text/html; charset=utf-8' });
      res.end(`
        <!DOCTYPE html>
        <html>
        <head><title>404 Not Found</title></head>
        <body style="font-family: monospace; padding: 20px;">
          <h1>404 Not Found</h1>
          <p>${pathname}</p>
          <p><a href="/x3star-dashboard.html">← Back to Dashboard</a></p>
        </body>
        </html>
      `);
      return;
    }

    // Determine content type
    const ext = path.extname(filepath).toLowerCase();
    const contentType = MIME_TYPES[ext] || 'application/octet-stream';

    // Production headers
    const headers = {
      'Content-Type': contentType,
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, OPTIONS',
      'Cache-Control': 'public, max-age=3600', // 1 hour
      'X-Content-Type-Options': 'nosniff',
      'X-Frame-Options': 'SAMEORIGIN',
      'X-XSS-Protection': '1; mode=block',
      'Referrer-Policy': 'strict-origin-when-cross-origin',
    };

    // Don't cache HTML files aggressively
    if (ext === '.html') {
      headers['Cache-Control'] = 'public, max-age=3600, must-revalidate';
    }

    // Serve file
    res.writeHead(200, headers);
    res.end(content);

    // Log
    console.log(`[${new Date().toISOString()}] ${req.method} ${pathname} - ${content.length} bytes`);
  });
});

server.listen(PORT, '0.0.0.0', () => {
  console.log(`\n✅ X3 Production Server`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`🌐 Listening on: http://0.0.0.0:${PORT}`);
  console.log(`📁 Serving: ${PUBLIC_DIR}`);
  console.log(`🔗 Home: http://localhost:${PORT}/x3star-dashboard.html`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);
});

process.on('SIGINT', () => {
  console.log('\n\n✋ Shutting down production server...');
  server.close(() => {
    console.log('Server stopped.');
    process.exit(0);
  });
});
