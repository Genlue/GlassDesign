// 从运行中的画廊导出**可点的**静态预览（不含 WASM 交互）。
//
// 与旧版的区别：旧版只 dump 当前分区（基础），左栏是死的；这里把 8 个分区逐个切过去抓，
// 合成一个文件 + 一段内联脚本负责「换分区 / 换明暗」。仍然不加载 wasm、不发请求，
// 所以 `file://` 双击也能用。
//
// 用法：node scripts/build-static.mjs [url] [out]（需 8777 端口在跑）
import { spawn } from 'node:child_process';
import { existsSync, writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const CHROME_CANDIDATES = [
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
];
const chrome = CHROME_CANDIDATES.find((p) => existsSync(p));
if (!chrome) throw new Error('找不到 Chrome/Edge');

const url = process.argv[2] || 'http://127.0.0.1:8777/index.html';
const out = process.argv[3] || 'index.html';
const port = 8791;
const profile = mkdtempSync(join(tmpdir(), 'gd-static-'));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const proc = spawn(chrome, [
  '--headless=new', '--disable-gpu', '--no-first-run', '--hide-scrollbars',
  '--window-size=1440,900',
  `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, 'about:blank',
], { stdio: 'ignore' });

async function waitDevtools() {
  for (let i = 0; i < 120; i++) {
    try {
      if ((await fetch(`http://127.0.0.1:${port}/json/version`)).ok) return;
    } catch {}
    await sleep(100);
  }
  throw new Error('DevTools 未就绪');
}

let ws;
let seq = 0;
const pending = new Map();

async function connect() {
  const page = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page');
  if (!page) throw new Error('没有 page target');
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((res, rej) => {
    ws.addEventListener('open', res, { once: true });
    ws.addEventListener('error', rej, { once: true });
  });
  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const p = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? p.reject(new Error(JSON.stringify(msg.error))) : p.resolve(msg.result);
    }
  });
}

const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const id = ++seq;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });

const ev = async (expression) => {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.text + ' ' + (r.exceptionDetails.exception?.description || ''));
  return r.result.value;
};

// 等某个条件成立（wasm 首帧、分区切换后的重渲染）
async function until(expr, label, timeout = 20000) {
  const t0 = Date.now();
  while (Date.now() - t0 < timeout) {
    if (await ev(expr)) return;
    await sleep(120);
  }
  throw new Error(`等待超时：${label}`);
}

async function main() {
  await waitDevtools();
  await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`, { method: 'PUT' });
  await sleep(400);
  await connect();
  await send('Runtime.enable');
  await send('Page.enable');

  await until(`document.querySelectorAll('.c-nav__item').length >= 8`, '左栏渲染');
  await until(`!!document.querySelector('.l-stage') && document.querySelector('.l-stage').children.length > 0`, '首个分区渲染');

  // 1) 逐个分区切换 + 抓 .l-stage 内容
  const labels = await ev(`[...document.querySelectorAll('.c-nav__item')].map(e => e.textContent.trim())`);
  const sections = [];
  for (const label of labels) {
    await ev(`(() => {
      const it = [...document.querySelectorAll('.c-nav__item')].find(e => e.textContent.trim() === ${JSON.stringify(label)});
      it.click();
      return true;
    })()`);
    // 等这一分区真的换上来：比对首个子元素的外层 HTML 变化
    await until(
      `(() => {
        const st = document.querySelector('.l-stage');
        if (!st || !st.children.length) return false;
        const it = [...document.querySelectorAll('.c-nav__item')].find(e => e.textContent.trim() === ${JSON.stringify(label)});
        return it && it.getAttribute('aria-selected') === 'true';
      })()`,
      `切换到「${label}」`
    );
    await sleep(260);
    const html = await ev(`document.querySelector('.l-stage').innerHTML`);
    sections.push({ label, html });
    process.stdout.write(`  · ${label} ${(Buffer.byteLength(html) / 1024).toFixed(0)} KB\n`);
  }

  // 2) 合成：直接在页面里把 .l-stage 的内容换成分区块，然后在**同一次求值**里取整页 HTML。
  //    分两步取会有风险：Rust 的反应式系统可能在两次调用之间重渲染，把注入的内容抹掉。
  const blocks = sections
    .map((s, i) => `<section class="gd-sec" data-sec="${s.label}"${i === 0 ? '' : ' hidden'}>${s.html}</section>`)
    .join('\n');

  let shell = await ev(`(() => {
    document.querySelector('.l-stage').innerHTML = ${JSON.stringify(blocks)};
    return document.documentElement.outerHTML;
  })()`);
  shell = shell.replace(/<script type="module">[\s\S]*?<\/script>/g, '');
  shell = shell.replace(/<script id="gd-engine">[\s\S]*?<\/script>/g, '');

  const style = `<style>
    /* 静态预览的换页规则：hidden 通常由 UA 样式表生效，但作者样式可能覆盖，这里写死。 */
    .gd-sec[hidden] { display: none !important; }
  </style>`;

  const script = `<script id="gd-static-nav">
(function () {
  var secs = [].slice.call(document.querySelectorAll('.gd-sec'));
  var items = [].slice.call(document.querySelectorAll('.c-nav__item'));
  function show(i) {
    secs.forEach(function (s, k) { s.hidden = k !== i; });
    items.forEach(function (it, k) { it.setAttribute('aria-selected', k === i ? 'true' : 'false'); });
    var stage = document.querySelector('.l-stage');
    if (stage) stage.scrollTop = 0;
    window.scrollTo(0, 0);
  }
  items.forEach(function (it, i) {
    it.addEventListener('click', function (e) { e.preventDefault(); show(i); });
    it.addEventListener('keydown', function (e) {
      if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); show(i); }
    });
  });
  // 明暗：真应用里是 Theme 服务改 <html data-theme>，这里直接翻属性（两套 CSS 都在）。
  var t = document.querySelector('[aria-label="切换明暗"]');
  if (t) t.addEventListener('click', function () {
    var r = document.documentElement;
    r.setAttribute('data-theme', r.getAttribute('data-theme') === 'dark' ? 'light' : 'dark');
  });
  // 语言切换靠 Rust 的 Strings 表，静态页做不到 —— 标成不可用，而不是让它看起来能点。
  var seg = document.querySelector('.c-seg');
  var cap = seg && seg.closest('.c-capsule');
  if (cap) {
    cap.setAttribute('aria-disabled', 'true');
    cap.setAttribute('title', '静态预览里不可用（到实时画廊切语言）');
    cap.style.opacity = '.45';
    cap.style.pointerEvents = 'none';
  }
  show(0);
})();
</script>`;

  let html = shell;
  html = html.replace('</head>', `${style}\n</head>`);
  html = html.replace('</body>', `${script}\n</body>`);
  html = html.replace(
    '<!DOCTYPE html>',
    `<!DOCTYPE html>\n<!-- 静态预览（可点）：8 个分区由 应用/ui 的 DOM 逐个导出，左栏与明暗可用；\n     控件自身的交互是冻结的（没有 WASM）。重新生成：node scripts/build-static.mjs（需 8777 在跑） -->`
  );

  writeFileSync(out, html, 'utf8');
  console.log(`${out} · ${(Buffer.byteLength(html) / 1024).toFixed(0)} KB · ${sections.length} 个分区`);
}

main()
  .then(() => { proc.kill(); process.exit(0); })
  .catch((e) => { console.error('导出失败：', e.message); proc.kill(); process.exit(1); });
