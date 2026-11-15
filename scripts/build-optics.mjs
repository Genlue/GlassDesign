// 从 glass-schema.js 的默认值生成每个玻璃面的 CSS 变量。
// Rust 只覆写变量；没覆写时用这里的默认值。
import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const schemaPath = resolve(ROOT, 'scripts/glass/glass-schema.js');
const schema = await import(pathToFileURL(schemaPath).href);

const SURFACES = [
  ['chrome', '.c-glass-chrome'],
  ['panel', '.c-glass-panel'],
  ['capsule', '.c-capsule'],
  ['dock', '.c-dock__pill'],
  ['menu', '.c-menu'],
  ['toast', '.c-toast'],
  ['sheet', '.c-glass-sheet'],
  ['thumb', '.c-slider__thumb'],
];

const color = (v) => (v === 'glass' ? 'rgb(var(--glass-rgb))' : v);
const fixed = (v, n) => Number(v).toFixed(n);

const blocks = SURFACES.map(([id, cls]) => {
  const o = schema.glassSurfaceDefaults(id);
  return `${cls} {
  --lg-band: ${o.band};
  --lg-amount: ${o.amount};
  --lg-amount-k: ${fixed(Math.min(1, o.amount / 10), 3)};
  --lg-depth: ${o.depth};
  --lg-dispersion: ${o.dispersion};
  --lg-vibrancy: ${o.vibrancy};
  --lg-coating: ${o.coating};
  --lg-coating-color: ${color(o.coatingColor)};
  --lg-stroke-half: ${o.strokeHalf};
  --lg-stroke-feather: ${o.strokeFeather};
  --lg-stroke-alpha: ${o.strokeAlpha};
  --lg-falloff: ${o.falloff};
  --lg-light: ${o.light};
  --lg-sheen: ${o.sheen};
  --lg-sheen-depth: ${o.sheenDepth};
  --glass-blur-k: ${o.blurK};
}`;
});

const out = `/* 由 scripts/build-optics.mjs 生成，请勿手改。
   来源：scripts/glass/glass-schema.js 的 8 套默认参数。 */
${blocks.join('\n\n')}
`;

const target = resolve(ROOT, '控件/css/src/optics.css');
writeFileSync(target, out, 'utf8');
console.log(`optics.css · ${(statSync(target).size / 1024).toFixed(1)} KB`);
