# GlassDesign

Windows 桌面界面库：**Tauri 2 + Rust + Leptos（CSR）+ 构建期预计算的液态玻璃**。仓库本身即可作为依赖源，用 git tag 声明引用（写法见 [`docs/05-接入手册.md`](docs/05-接入手册.md)）；也可以整个仓库当作参考实现。

界面用 `view!` 拼，运行时不碰 DOM、不碰测量、不注入 JavaScript。玻璃的模糊与折射全部在构建期算成静态 SVG filter，运行期只剩一条 `backdrop-filter`。

---

## 四个目录 = 四层

| 目录 | 角色 | 谁能依赖谁 |
|---|---|---|
| `控件/` | **界面库本体**，crate `glass-design` | 只依赖自己 `Cargo.toml` 里的 crates |
| `host/` | 宿主胶水，crate `glass-design-host`：窗口装配、桌面截屏、壁纸 | 依赖 tauri + windows，不依赖 `控件/` |
| `应用/` | 使用者：`ui/` 画廊（wasm）、`src-tauri/` 宿主 | 依赖 `控件/`、`host/` 与 `scripts/` 的产物 |
| `scripts/` | 构建期工具：CSS、贴图、图标、打包、截图 | 只读 `控件/`，写它的产物 |

依赖是单向的：`scripts → 控件 ← 应用 → host`。`控件/` 不许反向依赖任何东西——它整个文件夹拷进别的项目就能用。

```text
GlassDesign/
├── 控件/                         ← 库本体（也能整目录拷走）
│   ├── Cargo.toml                  crate glass-design
│   ├── src/
│   │   ├── base/                   install / theme / overlay / anchor / virtual_window / icon / i18n
│   │   ├── layout/                 shell / toolbar / surfaces / glass_panel
│   │   └── widgets/                104 个公开组件，一个文件一族
│   ├── css/
│   │   ├── src/                    tokens · optics · base · layout + components/（64 个分区）
│   │   └── glass.css               展开后的单文件（build-css 生成）
│   └── assets/glass-defs.svg       预计算位移/高光贴图 + 静态 filter
├── 应用/
│   ├── ui/                        Leptos 画廊（九个分区，含「调节」）
│   └── src-tauri/                 无边框窗口 + 背景截屏命令
├── host/                          宿主胶水 crate（窗口装配 / 截屏 / 壁纸）
├── scripts/                       构建与验证工具（见 docs/03）
├── docs/                          四篇：架构 / 组件契约 / 构建验证 / 作为库分发
├── template/                      cargo-generate 模板（三步起一个新项目）
├── index.html                     ★ 双击即用的单文件预览（wasm 内联，1.42 MB）
└── index-static.html              只读快照（无 JS，仅比对版式）
```

---

## 三分钟跑起来

```sh
# 1. 样式与材质（构建期一次性计算）
node scripts/build-optics.mjs
node scripts/build-glass.mjs
node scripts/build-css.mjs

# 2. wasm（release 默认带 24 MB 符号名，两个 --remove-* 不能省）
cargo build -p glass-design-gallery --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir 应用/ui/dist target/wasm32-unknown-unknown/release/glass_design_gallery.wasm

# 3. 看
node scripts/preview.mjs                 # http://127.0.0.1:8777 —— 热改热看
# 或者打包成单文件，双击就能点：
wasm-bindgen --target no-modules --no-typescript --remove-name-section --remove-producers-section \
  --out-name gd --out-dir 应用/ui/dist/nm target/wasm32-unknown-unknown/release/glass_design_gallery.wasm
node scripts/build-standalone.mjs        # → index.html

# 4. 宿主
cargo build -p glass-design-app && ./target/debug/glass-design-app.exe
```

只需改样式时，跑第 1、3 步就够（第 3 步要分别重跑两次 wasm-bindgen 与 standalone）。

---

## 液态玻璃怎么实现的

材质不是运行时算的，是**构建期算完画成贴图**：

```text
scripts/glass/{schema,shape,optics}.js
        │  build-glass.mjs
        ├──▶ 位移图 + 高光图（PNG → base64）
        ├──▶ 控件/assets/glass-defs.svg   〈filter id="gd-glass-*"〉
        └──▶ components/63-glass-svg.css  选择器 + backdrop-filter: url(#…)
install() ──▶ <style id="gd-styles"> + <div id="gd-glass-defs">
组件只写 class / data-glass ──▶ 浏览器自己套 filter
```

两条硬约束：

1. **贴图尺寸必须等于元素尺寸。** Chromium 的 `feImage` 不能可靠拉伸预计算贴图，所以玻璃按 11 个固定档位生成（胶囊 34/112/180、拇指 24×16、面板三档、菜单两档、Sheet、Toast）；不在档位里的走 `Auto`：纯 CSS 涂层，没有位移折射。
2. **玻璃折的是 `#wallpaper`。** `backdrop-filter` 只能采窗口内的像素，窗口外的桌面它看不见。所以页面最底层必须是一张"桌面快照"：宿主（Tauri）实时截屏喂进来，浏览器预览用合成桌面顶上。内容面留 94% 不透明，桌面只透上来一层气氛。

---

## 硬规矩

1. **`控件/` 是唯一复制单元**：不许 import `应用/`、`scripts/`、`docs/`，不许依赖运行期文件路径（样式与贴图一律 `include_str!`）。
2. **外观只走令牌与类名**：Rust 里不出现字面色值、字号、圆角、阴影；动态几何（`top/left/height`）除外。
3. **材质只由固定尺寸档触发**：组件只写 `class` / `data-glass`，运行时不计算滤镜。
4. **任何 `position: fixed` 的浮层必须走 `Portal`**：留在组件 DOM 里会踩 `transform` / `backdrop-filter` 的包含块。
5. **虚拟化行高必须固定**，且 CSS 与 Rust 用同一个数。
6. **只追加，不改既有分区规则**：需要补充就追加到对应 `components/*.css` 末尾并写清原因，不动 `components.css` 的 import 顺序。
7. **`prelude` 是唯一公开出口**：新增组件不在 `lib.rs` 导出就等于没做。

---

## 作为依赖使用

```sh
cargo add glass-design --git https://github.com/Genlue/GlassDesign --tag v0.8.2
cargo add leptos --features csr
cargo add wasm-bindgen
```

```rust
use leptos::prelude::*;
use glass_design::prelude::*;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() { bootstrap(App); }
```

三个依赖都要写在应用里：`#[component]` / `view!` 展开成 `::leptos::…` 绝对路径，re-export 替代不了声明。库 re-export 只用于统一 props 类型与版本。

**完整接入步骤、页面骨架、材质档位、宿主接线与排障，看 [`docs/05-接入手册.md`](docs/05-接入手册.md)**——那份文档是自包含的，可以直接发给新会话。其他写法（path / 纯 HTML）、`embedded` 开关、主题令牌与图标扩展，见 [`控件/README.md`](控件/README.md)。

---

## 文档

| 文档 | 回答什么 |
|---|---|
| [`docs/01-架构.md`](docs/01-架构.md) | 七条链怎么咬合：材质 / 主题 / 浮层 / 虚拟化 / 焦点 / 状态 / 文案 |
| [`docs/02-组件契约.md`](docs/02-组件契约.md) | 对外承诺什么：props、受控、插槽、浮层、主题、材质档、104 个组件与各自边界 |
| [`docs/03-构建与验证.md`](docs/03-构建与验证.md) | 怎么构建、产物是什么、怎么验证没弄坏、出问题查哪里 |
| [`docs/04-作为库分发.md`](docs/04-作为库分发.md) | 维护者向：将来真要对外发布时的流程、semver 与 0.x 双版本共存风险（当前未发布，不必看） |
| [`docs/05-接入手册.md`](docs/05-接入手册.md) | **新会话先读这份**：从零接入、页面骨架、材质档位、宿主接线、排障，自包含 |
| [`控件/README.md`](控件/README.md) | 库的四种引用方式与 API 概览 |

---

## 当前产物

| 项 | 值 |
|---|---|
| 版本 | `glass-design` 0.8.0 / `glass-design-host` 0.8.0 |
| 公开组件 | 104 个（`#[component]` 共 106 处，2 个为 `pub(crate)` / 私有，全部从 `prelude` 导出） |
| CSS | `tokens / optics / base / layout` + 64 个分区（63 个参与合并，`glass.css` 313 KB） |
| 材质档位 | 11 档 + `Auto` 兜底 |
| 图标 | 97 个（24 视框，线宽由 `--ic-stroke` 控制） |
| wasm | 3.42 MB（gzip 1.01 MB）+ 胶水 66 KB |
| 发布包 | 84 个文件，1.3 MiB；压缩 405.5 KiB（`cargo package --list` 实测） |
| 单文件预览 | `index.html` 1.42 MB（wasm gzip + base64 内联，`file://` 可跑） |
| 只读快照 | `index-static.html` 712 KB（九个分区可切，控件冻结） |
| 单测 | `cargo test -p glass-design --lib` → 30 passed |
| 平台 | Windows / WebView2（macOS、Linux 只承诺可用 + CSS 兜底） |
