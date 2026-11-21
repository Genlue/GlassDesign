# {{project-name}}

用 [glass-design](https://crates.io/crates/glass-design) 起的 Tauri 2 + Leptos（CSR）桌面应用骨架。

```text
{{project-name}}/
├── ui/                前端（wasm）。依赖 glass-design + leptos + wasm-bindgen
│   ├── src/lib.rs     bootstrap(App) + 一页 AppShell
│   ├── wallpaper.js   壁纸接线（host/assets/wallpaper.js 的副本）
│   └── index.html     最小壳
└── src-tauri/         宿主：无边框不透明窗口 + 壳层接线
```

> 仓库里这三份清单叫 `Cargo.toml.liquid`。cargo-generate 会把 `.liquid` 渲染掉、
> 后缀去掉（`README.md.liquid` → `README.md`），生成出来就是正常的 `Cargo.toml`。
> 这么做是因为 `[package] name` 里的 `{{project-name}}` 不是合法包名 —— 清单叫
> `Cargo.toml` 的话，谁把这个仓库当 git 依赖引进项目，cargo 递归扫检出目录时
> 就会撞上它并刷两行 `invalid character` 错误。生成出来的项目不受影响。

## 三步跑起来

```sh
# ① 生成（本模板假设你从仓库的 template/ 目录生成）
cargo generate --git <本仓库地址> template
cd {{project-name}}
```

```sh
# ② 编 wasm。wasm-bindgen CLI 的版本要跟解析出来的 crate 一致：
#    cargo tree -p {{project-name}}-ui | grep wasm-bindgen
rustup target add wasm32-unknown-unknown
cargo build -p {{project-name}}-ui --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript \
  --out-dir ui/pkg \
  target/wasm32-unknown-unknown/release/{{crate_name}}_ui.wasm
```

```sh
# ③ 起宿主
cargo run -p {{project-name}}
```

改了 Rust 代码要重跑第 ② 步再重开第 ③ 步。

## 只有一页东西要看懂

`ui/src/lib.rs` 里两行 import，各管一半：

| import | 拿到什么 |
|---|---|
| `use leptos::prelude::*;` | `view!` / `#[component]` / `RwSignal` / `IntoView`，框架本体 |
| `use glass_design::prelude::*;` | 组件（`AppShell` / `Card` / `Switch`…）、`bootstrap`、`Theme`、`IconName` |

`ui/Cargo.toml` 里 `leptos` 与 `wasm-bindgen` 都要声明。`#[component]` / `view!` 展开成 `::leptos::…` 绝对路径，`#[wasm_bindgen]` 展开成 `::wasm_bindgen::…`——指向本 crate 根的外部 crate，库的 re-export 替代不了。re-export 只服务 `Signal` / `Callback` 这类 props 类型的版本锚点。

版本写 caret（`leptos = "0.8"`），别写 `=0.8.x`：库用 caret，锁死会各拿一份 leptos，`Signal` 变成两个类型，满屏 E0308。

## 关于依赖版本

模板声明的是 crates.io 上的 `glass-design = "0.8"` 与 `glass-design-host = "0.8"`，`ui/` 侧另有 `leptos = "0.8"` 与 `wasm-bindgen = "0.2"`。

**这套模板实测可编译**，离线也行：把上面两条 `glass-design*` 换成仓库里的 path 依赖（`控件/` 与 `host/`），`ui/` 与 `src-tauri/` 两侧的 `cargo check --offline` 都能过。这么做的原因只是 crates.io 缓存里没有已发布的 0.8；模板里保留 `"0.8"` 是给 `cargo generate` 出来的人用的，别把 path 写回模板。

## 壁纸是怎么来的

`ui/wallpaper.js` 是 `glass-design-host` 里 `WALLPAPER_JS` 的副本（源文件 `host/assets/wallpaper.js`）。`index.html` 用普通 `<script src="wallpaper.js">` 引入它，而 `tauri.conf.json` 的 `app.withGlobalTauri` 必须为 `true`，否则脚本拿不到 `window.__TAURI__`，不会打开宿主全铺模式（`data-shell="host"`）。

- 在宿主里：它检测到 `window.__TAURI__` 后在 `<html>` 上写 `data-shell="host"`（去掉浏览器预览的 10px 外框），并保持 `#wallpaper` 使用 `tokens.css` 的内置合成渐变壁纸（`data-source="demo"`）。
- 在浏览器里：同样使用内置合成壁纸，并保留 10px 浮窗预览外框。
- 不进行桌面截屏，也不挂载 `WDA_EXCLUDEFROMCAPTURE`，因此远程桌面、系统截图、录屏与任务栏预览均正常工作。

真实项目里不必维护这份副本：`glass_design_host::WALLPAPER_JS` 是个 `&str` 常量，可以在构建期落盘到 `ui/wallpaper.js`（如加进 build-dependencies 由 `build.rs` 写出，和仓库里 `scripts/sync-host-assets.mjs` 一个思路）。改了 `host/assets/wallpaper.js` 记得同步这份副本。

## 打包

`tauri.conf.json` 里 `bundle.active` 是 `false`，先能跑再谈发布。要出安装包：把 `bundle.active` 改 `true`，再换一套正式图标（`cargo tauri icon path/to/icon.png`）。

`src-tauri/icons/icon.ico` 不是「打包才要」的东西：tauri-build 会无条件为 Windows 生成资源文件（`.res`），缺了这个 ico 连 `cargo check` 都过不去。模板里带了一份占位图标，别删；`bundle.icon` 保持空数组即可，Tauri 会回落到默认的 `icons/` 查找。

`tauri.conf.json` 的 `frontendDist` 指向 `../ui`（不是 `../ui/dist`），这是个有意的取舍：新克隆下来直接 `cargo check` 就能过，不必先跑一次 wasm 构建。代价是 `ui/src/*.rs`、`ui/Cargo.toml` 也会被嵌进二进制——功能无碍，但如果你介意把源码打进产物，就在构建流程里把 `index.html` / `wallpaper.js` / `pkg/` 拷到 `ui/dist/`，再把 `frontendDist` 改指过去（仓库里的 `应用/` 就是这么做的）。
