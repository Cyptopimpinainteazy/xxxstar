// Static file server for the App Store frontend: no dependencies.
// It replaces live-server, whose dependency tree carried high-severity advisories
// for a job this small (serve index.html and src/ to a browser).
//   node serve.mjs [--port 3001] [--host localhost]
import { createServer } from 'node:http';
import { readFile, realpath, stat } from 'node:fs/promises';
import { extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)));
const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > -1 && process.argv[i + 1] ? process.argv[i + 1] : fallback;
};
const PORT = Number(arg('port', process.env.PORT || 3001));
const HOST = arg('host', process.env.HOST || 'localhost');
const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.tsx': 'text/babel; charset=utf-8',
  '.ts': 'text/babel; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
};

createServer(async (req, res) => {
  try {
    const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    let file = join(ROOT, normalize(path));
    // Never serve outside the frontend directory, or its dependencies.
    if ((file !== ROOT && !file.startsWith(ROOT + sep)) || file.includes(`${sep}node_modules${sep}`)) {
      res.writeHead(403).end('forbidden');
      return;
    }
    if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
    // Resolve directory and file symlinks before enforcing the document boundary.
    file = await realpath(file);
    if (!file.startsWith(ROOT + sep) || file.includes(`${sep}node_modules${sep}`)) {
      res.writeHead(403).end('forbidden');
      return;
    }
    const body = await readFile(file);
    res.writeHead(200, { 'Content-Type': TYPES[extname(file)] || 'application/octet-stream' }).end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(PORT, HOST, () => console.log(`x3-app-store frontend on http://${HOST}:${PORT}/`));
