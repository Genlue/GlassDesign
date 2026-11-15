// 从运行中的画廊导出静态视觉快照（不含 WASM 交互）。
// 用法：node scripts/build-static.mjs [url] [out]
import { spawn } from 'node:child_process';
import { existsSync, writeFileSync } from 'node:fs';

const CHROME_CANDIDATES = [
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
];
const chrome = CHROME_CANDIDATES.find((p) => existsSync(p));
if (!chrome) throw new Error('找不到 Chrome/Edge');

const url = process.argv[2] || 'http://127.0.0.1:8777/index.html';
const out = process.argv[3] || 'index.html';

const proc = spawn(chrome, [
  '--headless=new', '--disable-gpu', '--no-first-run',
  '--dump-dom', '--virtual-time-budget=4000', url,
], { stdio: ['ignore', 'pipe', 'ignore'] });

let html = '';
proc.stdout.on('data', (d) => { html += d.toString(); });
proc.on('close', (code) => {
  if (code !== 0 || !html) throw new Error(`导出失败：exit ${code}`);
  html = html.replace(/<script type="module">[\s\S]*?<\/script>/g, '');
  html = html.replace(/<script id="gd-engine">[\s\S]*?<\/script>/g, '');
  html = html.replace(
    '<!DOCTYPE html>',
    '<!DOCTYPE html>\n<!-- 静态视觉快照：由 应用/ui 的 DOM 导出，不含 WASM 交互。\n     重新生成：node scripts/build-static.mjs（需 8777 端口在跑） -->'
  );
  writeFileSync(out, html, 'utf8');
  console.log(`${out} · ${(Buffer.byteLength(html) / 1024).toFixed(0)} KB`);
});
