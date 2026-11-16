// 一行命令看**实时画廊**（完整交互：菜单 / 对话框 / 编辑 / 键盘）。
//
// 用法：node scripts/preview.mjs [port]
// 与根目录 index.html 的分工：
//   · index.html      —— 静态预览，双击即看、8 个分区可切，但控件交互是冻结的
//   · 这个脚本         —— 起一个本地服务指向 应用/ui/dist，跑真正的 wasm 应用
//
// 为什么必须走 http：ES module + fetch(wasm) 在 file:// 下会被 CORS 拦掉。
import { createServer } from 'node:http';
import { createReadStream, existsSync, statSync } from 'node:fs';
import { extname, join, normalize, resolve } from 'node:path';

const root = resolve('应用/ui/dist');
const port = Number(process.argv[2] || 8777);

if (!existsSync(join(root, 'index.html'))) {
  console.error(`找不到 ${root}/index.html —— 先构建：\n  cargo build -p glass-design-gallery --target wasm32-unknown-unknown --release\n  wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir 应用/ui/dist target/wasm32-unknown-unknown/release/glass_design_gallery.wasm`);
  process.exit(1);
}

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.css': 'text/css; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.json': 'application/json; charset=utf-8',
};

const server = createServer((req, res) => {
  const raw = decodeURIComponent((req.url || '/').split('?')[0]);
  // 目录穿越：normalize 之后必须仍在 root 里
  const target = normalize(join(root, raw === '/' ? '/index.html' : raw));
  if (!target.startsWith(root)) {
    res.writeHead(403).end('forbidden');
    return;
  }
  if (!existsSync(target) || !statSync(target).isFile()) {
    res.writeHead(404).end('not found');
    return;
  }
  res.writeHead(200, {
    'content-type': MIME[extname(target).toLowerCase()] || 'application/octet-stream',
    // wasm-bindgen 的胶水靠这个做流式编译；本地开发够用
    'cache-control': 'no-store',
  });
  createReadStream(target).pipe(res);
});

server.listen(port, '127.0.0.1', () => {
  console.log(`实时画廊：http://127.0.0.1:${port}/index.html`);
  console.log('（Ctrl+C 停止；静态预览请直接双击仓库根目录的 index.html）');
});
