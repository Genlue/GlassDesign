// 连接已运行的 WebView2/Chrome CDP 端点，截图并读取状态。
// 用法：node scripts/cdp-shot.mjs <port> <out.png>
import { writeFileSync } from 'node:fs';

const port = Number(process.argv[2] || 9223);
const out = process.argv[3] || '.shots-tauri/tauri.png';

const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = targets.find((t) => t.type === 'page');
if (!page) throw new Error('没有 page target');

const ws = new WebSocket(page.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
const errors = [];

const send = (method, params = {}) => new Promise((resolve, reject) => {
  const mid = ++id;
  pending.set(mid, { resolve, reject });
  ws.send(JSON.stringify({ id: mid, method, params }));
});

await new Promise((resolve, reject) => {
  ws.addEventListener('open', resolve, { once: true });
  ws.addEventListener('error', reject, { once: true });
  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const p = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? p.reject(new Error(JSON.stringify(msg.error))) : p.resolve(msg.result);
    } else if (msg.method === 'Runtime.exceptionThrown') {
      errors.push(msg.params.exceptionDetails?.exception?.description || msg.params.exceptionDetails?.text);
    } else if (msg.method === 'Log.entryAdded' && msg.params.entry.level === 'error') {
      errors.push(msg.params.entry.text);
    }
  });
});

await send('Page.enable');
await send('Runtime.enable');
await send('Log.enable');
await new Promise((r) => setTimeout(r, 1200));

const state = await send('Runtime.evaluate', {
  expression: `(() => ({
    title: document.title,
    filters: document.querySelectorAll('#gd-glass-defs filter').length,
    glass: document.querySelectorAll('.c-capsule').length,
    bf: document.querySelector('.c-capsule') ? getComputedStyle(document.querySelector('.c-capsule')).backdropFilter : '',
    theme: document.documentElement.dataset.theme,
    size: document.querySelector('.c-capsule') ? getComputedStyle(document.querySelector('.c-capsule')).width : ''
  }))()`,
  returnByValue: true,
});

const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
writeFileSync(out, Buffer.from(shot.data, 'base64'));

console.log(JSON.stringify({ state: state.result.value, errors }, null, 2));
ws.close();
