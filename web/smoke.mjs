import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const assets = resolve(process.argv[2] ?? 'target/glass-web-smoke');
const reducedMotion = process.env.GD_REDUCED_MOTION === '1';
const edge = process.argv[3] ?? join(process.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)',
  'Microsoft/Edge/Application/msedge.exe');
const routes = new Map([
  ['/smoke.js', ['text/javascript', fileURLToPath(new URL('./smoke.js', import.meta.url))]],
  ['/glass_web.js', ['text/javascript', join(assets, 'glass_web.js')]],
  ['/glass_web_bg.wasm', ['application/wasm', join(assets, 'glass_web_bg.wasm')]],
  ['/glass.css', ['text/css', join(assets, 'glass.css')]],
]);
const defs = await readFile(join(assets, 'glass-defs.svg'), 'utf8');
const profile = await mkdtemp(fileURLToPath(new URL('../target/glass-web-edge-', import.meta.url)));
let finish;
const result = new Promise((resolve) => { finish = resolve; });
const server = createServer(async (req, res) => {
  if (req.url === '/result' && req.method === 'POST') {
    let text = '';
    for await (const part of req) text += part;
    res.end('ok');
    finish(JSON.parse(text));
  } else if (req.url === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(`<!doctype html><meta charset="utf-8"><title>GlassDesign smoke</title>
      <link rel="icon" href="data:,"><link rel="stylesheet" href="/glass.css">
      ${defs}<main></main><script type="module" src="/smoke.js"></script>`);
  } else {
    const route = routes.get(req.url);
    if (!route) return res.writeHead(404).end();
    try {
      const bytes = await readFile(route[1]);
      res.writeHead(200, { 'Content-Type': route[0] }).end(bytes);
    } catch (error) {
      res.writeHead(500).end(String(error));
    }
  }
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const browser = spawn(edge, ['--headless=new', '--disable-gpu', '--no-first-run',
  ...(reducedMotion ? ['--force-prefers-reduced-motion'] : []),
  `--user-data-dir=${profile}`, `http://127.0.0.1:${server.address().port}/`],
  { stdio: 'ignore', windowsHide: true });
browser.on('error', (error) => finish({ error: String(error) }));
browser.on('exit', (code) => finish({ error: `Browser exited early (${code})` }));
const timeout = setTimeout(() => finish({ error: 'Timed out waiting for the browser smoke test' }), 30_000);
try {
  const report = await result;
  if (report.error) throw new Error(report.error);
  console.log(`GlassDesign browser smoke passed: ${report.checks} checks`);
} finally {
  clearTimeout(timeout);
  browser.kill();
  server.close();
}
