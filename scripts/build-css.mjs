// 把 控件/css/src 的 @import 展开成单文件 控件/css/glass.css。
// 顺序是契约：tokens → base → layout → components → **optics 收尾**。
// optics 必须最后：它是每个玻璃面的最终参数（schema 的产物）。
// 组件的材质分档（17-materials）也写 --glass-blur-k，那是"档位默认值"；
// 相等特异度下后写的赢，所以 optics 排前面会被整片盖掉（曾导致 chrome/panel/menu/sheet
// 四块面的模糊比 schema 小/大一大截）。
import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { dirname, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = resolve(ROOT, '控件', 'css', 'src');

function inline(abs, ancestors = []) {
  if (ancestors.includes(abs)) throw new Error(`CSS 循环：${[...ancestors, abs].join(' → ')}`);
  const text = readFileSync(abs, 'utf8');
  const next = text.replace(
    /@import\s+(?:url\(\s*)?['"]([^'"]+)['"]\s*\)?\s*;/g,
    (_, target) => inline(resolve(dirname(abs), target), [...ancestors, abs])
  );
  const rel = relative(SRC, abs).split(sep).join('/');
  return `\n/* ===== ${rel} ===== */\n${next}`;
}

const parts = ['tokens.css', 'base.css', 'layout.css', 'components.css', 'optics.css'].map((f) =>
  inline(resolve(SRC, f))
);

const out = `/* 由 scripts/build-css.mjs 生成，请勿手改。
   源：控件/css/src/{tokens,optics,base,layout,components}.css
   顺序是契约：tokens → base → layout → components（64 个分区）→ optics（各面最终参数，必须最后）。 */
${parts.join('\n')}
`;

const target = resolve(ROOT, '控件', 'css', 'glass.css');
writeFileSync(target, out, 'utf8');
console.log(`glass.css · ${(statSync(target).size / 1024).toFixed(1)} KB`);
