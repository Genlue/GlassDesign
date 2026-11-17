// 把 host crate 里的壁纸接线脚本同步到画廊的 dist。
// 为什么外置：截屏 → 亮度校验 → 移动/缩放去抖 → 慢轮询，这段是**宿主**的职责，
// 不属于前端库；应用只需要把它放到 <script src="wallpaper.js"> 能找到的地方。
// 源只有一份：host/assets/wallpaper.js（同时被 host crate 的 WALLPAPER_JS 内嵌）。
import { readFile, writeFile, stat } from 'node:fs/promises';
import { dirname, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = resolve(ROOT, 'host', 'assets', 'wallpaper.js');
const DEST = resolve(ROOT, '应用', 'ui', 'dist', 'wallpaper.js');

const code = await readFile(SRC, 'utf8');
await writeFile(DEST, code, 'utf8');

const { size } = await stat(DEST);
const rel = relative(ROOT, DEST).split(sep).join('/');
console.log(`wallpaper.js · ${(size / 1024).toFixed(1)} KB → ${rel}`);
