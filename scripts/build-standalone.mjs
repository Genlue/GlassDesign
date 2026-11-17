// 把「画廊 + wasm」压成**一个可以双击打开**的 index.html。
//
// 为什么需要这一步：`file://` 下浏览器既不让 `<script type="module">` 去 import
// 本地文件（CORS），也不让 `fetch()` 读本地 wasm。想做「双击就能点、能看动效」的
// 预览，只有一条路：把胶水和 wasm 一起塞进 HTML，用 `initSync` 直接实例化。
//
// wasm 先 gzip 再 base64（1.0 MB → 1.35 MB 文本），页面里用 DecompressionStream
// 解回来；不压的话 base64 是 4.5 MB。
//
// 用法：node scripts/build-standalone.mjs
// 前置：cargo build -p glass-design-gallery --target wasm32-unknown-unknown --release
//       wasm-bindgen --target web ... --out-dir 应用/ui/dist          （给本地服务器用）
//       wasm-bindgen --target no-modules --out-name gd --out-dir 应用/ui/dist/nm（给这里用）
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { gzipSync } from 'node:zlib';

const TEMPLATE = '应用/ui/dist/index.html';
const GLUE = '应用/ui/dist/nm/gd.js';
const WASM = '应用/ui/dist/nm/gd_bg.wasm';
const OUT = 'index.html';

for (const f of [TEMPLATE, GLUE, WASM]) {
  if (!existsSync(f)) {
    console.error(`缺少 ${f}\n先跑：\n  cargo build -p glass-design-gallery --target wasm32-unknown-unknown --release\n  wasm-bindgen --target no-modules --no-typescript --remove-name-section --remove-producers-section --out-name gd --out-dir 应用/ui/dist/nm target/wasm32-unknown-unknown/release/glass_design_gallery.wasm`);
    process.exit(1);
  }
}

const html = readFileSync(TEMPLATE, 'utf8');
const glue = readFileSync(GLUE, 'utf8').replace(/<\/script/gi, '<\\/script');
const raw = readFileSync(WASM);
const gz = gzipSync(raw, { level: 9 });
const b64 = gz.toString('base64');

const boot = `<script>
/* wasm-bindgen 的 no-modules 胶水。它把 __wbg_init / initSync 挂在全局 wasm_bindgen 上。 */
${glue}
</script>
<script>
/* 本地文件场景：wasm 以 base64(gzip) 内联，这里解压后直接同步实例化。
   整个页面没有任何 import / fetch —— 双击 index.html 就能跑。 */
(function () {
  var b64 = "${b64}";
  var bin = atob(b64);
  var bytes = new Uint8Array(bin.length);
  for (var i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  var fail = function (e) {
    document.body.insertAdjacentHTML('afterbegin',
      '<pre style="color:#ff6b6b;padding:20px;font:13px/1.6 monospace">WASM 启动失败：' +
      String(e) + '</pre>');
    console.error(e);
  };
  if (typeof DecompressionStream === 'function') {
    new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip')))
      .arrayBuffer()
      .then(function (buf) { wasm_bindgen.initSync({ module: buf }); })
      .catch(fail);
  } else {
    fail('这个浏览器没有 DecompressionStream（需要 Chrome/Edge 80+）');
  }
})();
</script>`;

const out = html.replace(
  /<script type="module">\s*import init from '\.\/glass_design_gallery\.js';\s*init\(\);\s*<\/script>/,
  boot
);

if (out === html) {
  console.error('没匹配到 init 脚本块 —— 模板结构和脚本对不上了');
  process.exit(1);
}

writeFileSync(OUT, out, 'utf8');
console.log(`${OUT} · ${(Buffer.byteLength(out) / 1024 / 1024).toFixed(2)} MB（wasm ${(raw.length / 1024 / 1024).toFixed(2)} MB → gzip ${(gz.length / 1024 / 1024).toFixed(2)} MB）`);
