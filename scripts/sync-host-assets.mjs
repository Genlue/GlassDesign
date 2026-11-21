// 把 host crate 里的壁纸/壳层接线脚本同步到画廊与模板目录。
// 源只有一份：host/assets/wallpaper.js（同时被 host crate 的 WALLPAPER_JS 内嵌）。
import { readFile, writeFile, stat } from 'node:fs/promises';
import { dirname, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = resolve(ROOT, 'host', 'assets', 'wallpaper.js');
const DESTS = [
  resolve(ROOT, '应用', 'ui', 'dist', 'wallpaper.js'),
  resolve(ROOT, 'template', 'ui', 'wallpaper.js'),
];

const code = await readFile(SRC, 'utf8');
for (const dest of DESTS) {
  await writeFile(dest, code, 'utf8');
  const { size } = await stat(dest);
  const rel = relative(ROOT, dest).split(sep).join('/');
  console.log(`wallpaper.js · ${(size / 1024).toFixed(1)} KB → ${rel}`);
}
