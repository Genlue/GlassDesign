// 把 控件/css/src 的 @import 展开成单文件 控件/css/glass.css。
// 顺序是契约：tokens → optics → base → layout → components(01-62)。
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

const parts = ['tokens.css', 'optics.css', 'base.css', 'layout.css', 'components.css'].map((f) =>
  inline(resolve(SRC, f))
);

const out = `/* 由 scripts/build-css.mjs 生成，请勿手改。
   源：控件/css/src/{tokens,optics,base,layout,components}.css
   顺序是契约：tokens → optics → base → layout → components(01-62)。 */
${parts.join('\n')}
`;

const target = resolve(ROOT, '控件', 'css', 'glass.css');
writeFileSync(target, out, 'utf8');
console.log(`glass.css · ${(statSync(target).size / 1024).toFixed(1)} KB`);
