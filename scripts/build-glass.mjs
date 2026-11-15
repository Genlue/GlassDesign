// 构建期生成静态玻璃资产：位移贴图 + 高光贴图 + SVG filter sprite。
// 运行时不需要 JavaScript、canvas 或 WebGL，只有一份静态 SVG 与 CSS。
import { readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { deflateSync } from 'node:zlib';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const GLASS = resolve(ROOT, 'scripts/glass');
const schema = await import(pathToFileURL(resolve(GLASS, 'glass-schema.js')).href);
const shapeMod = await import(pathToFileURL(resolve(GLASS, 'glass-shape.js')).href);
const optics = await import(pathToFileURL(resolve(GLASS, 'glass-optics.js')).href);

/* ---------------- PNG 编码（RGBA8，无依赖） ---------------- */

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const t = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])), 0);
  return Buffer.concat([len, t, data, crc]);
}

function encodePng(w, h, rgba) {
  const stride = w * 4;
  const raw = Buffer.alloc((stride + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (stride + 1)] = 0;
    Buffer.from(rgba.buffer, rgba.byteOffset + y * stride, stride).copy(raw, y * (stride + 1) + 1);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/* ---------------- 形状目录 ---------------- */

// 形状固定：每个面取一个代表尺寸。尺寸变化时由 preserveAspectRatio="none" 拉伸。
// 半径按该面的真实圆角给，不按比例缩放。
const SHAPES = [
  { id: 'capsule-34', surface: 'capsule', w: 34, h: 34, r: 17, css: '.c-capsule--round', blur: 'var(--m-popover-blur)' },
  { id: 'capsule-112', surface: 'capsule', w: 112, h: 34, r: 17, css: '.c-capsule[data-size="112"]', blur: 'var(--m-popover-blur)' },
  { id: 'capsule-180', surface: 'capsule', w: 180, h: 34, r: 17, css: '.c-capsule[data-size="180"]', blur: 'var(--m-popover-blur)' },
  { id: 'thumb', surface: 'thumb', w: 24, h: 16, r: 8, css: '.c-slider__thumb', blur: 'var(--thumb-blur-px)' },
];

const MAX_MAP = 256;

function buildOne(s) {
  const shape = shapeMod.resolveGlassShape({ w: s.w, h: s.h, radius: s.r });
  const p = schema.glassSurfaceDefaults(s.surface);
  const [mw, mh] = optics.glassMapSize(shape, MAX_MAP);

  const dmap = optics.glassDisplacementField(shape, p, mw, mh);
  const hmap = optics.glassHighlightField(shape, p, mw, mh);
  const dUri = `data:image/png;base64,${encodePng(mw, mh, dmap).toString('base64')}`;
  const hUri = `data:image/png;base64,${encodePng(mw, mh, hmap).toString('base64')}`;
  const scale = optics.glassDisplacementScale(p.amount, p.band);

  const filter = `<filter id="gd-glass-${s.id}" x="0" y="0" width="${s.w}" height="${s.h}" filterUnits="objectBoundingBox" primitiveUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
  <feImage href="${dUri}" result="dmap" x="0" y="0" width="${s.w}" height="${s.h}" preserveAspectRatio="none"/>
  <feDisplacementMap in="SourceGraphic" in2="dmap" xChannelSelector="R" yChannelSelector="G" scale="${scale.toFixed(3)}" result="lens"/>
  <feImage href="${hUri}" result="hl" x="0" y="0" width="${s.w}" height="${s.h}" preserveAspectRatio="none"/>
  <feComposite in="lens" in2="hl" operator="arithmetic" k1="0" k2="1" k3="0.55" k4="0"/>
</filter>`;

  return { ...s, mw, mh, filter, scale };
}

const built = SHAPES.map(buildOne);

const sprite = `<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0" aria-hidden="true" style="position:absolute;overflow:hidden;pointer-events:none">
<defs>
${built.map((b) => b.filter).join('\n')}
</defs>
</svg>
`;

const css = `/* 由 scripts/build-glass.mjs 生成，请勿手改。
   静态 SVG filter：位移 + 高光贴图在构建期算好，运行时没有 JavaScript。 */
${built
  .map(
    (b) => `${b.css} {
  width: ${b.w}px;
  height: ${b.h}px;
  justify-content: center;
  -webkit-backdrop-filter: blur(calc(${b.blur} * var(--glass-blur-k, 1))) saturate(calc(var(--lg-vibrancy) * 100%)) url(#gd-glass-${b.id});
  backdrop-filter: blur(calc(${b.blur} * var(--glass-blur-k, 1))) saturate(calc(var(--lg-vibrancy) * 100%)) url(#gd-glass-${b.id});
}`
  )
  .join('\n\n')}
`;

mkdirSync(resolve(ROOT, '控件/assets'), { recursive: true });
writeFileSync(resolve(ROOT, '控件/assets/glass-defs.svg'), sprite, 'utf8');
writeFileSync(resolve(ROOT, '控件/css/src/components/63-glass-svg.css'), css, 'utf8');
console.log(`glass-defs.svg · ${(statSync(resolve(ROOT, '控件/assets/glass-defs.svg')).size / 1024).toFixed(1)} KB`);
console.log(`63-glass-svg.css · ${built.length} 个 shape`);
for (const b of built) console.log(`  ${b.id}: ${b.w}x${b.h} r${b.r} → map ${b.mw}x${b.mh}, scale ${b.scale.toFixed(2)}`);
