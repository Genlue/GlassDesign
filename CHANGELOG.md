# 变更记录

本文件格式参照 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

规则：新增 prop / 组件 = minor；改 props 形状、删组件、改 DOM 或键盘契约、改贴图尺寸 = major；CSS 变量改名先加别名再删。

## [0.8.2] - 2026-11-17

只动打包形态，代码零变化。

### Fixed

- **模板清单污染下游的依赖解析**：`template/` 里三份 `Cargo.toml` 的
  `[package] name` 用 `{{project-name}}` 占位，不是合法包名。cargo 处理 **git 依赖**时
  会递归扫检出目录里所有名为 `Cargo.toml` 的文件（`PathSource::new_recursive`），
  不理会 `workspace.exclude` —— 于是任何人把这个仓库当 git 依赖引进来，首次拉取
  都会看到两行 `error: invalid character '{' in package name`。构建本身能过，但红字
  是真的。三份清单改名为 `Cargo.toml.liquid`：cargo 只认字面量 `Cargo.toml`，扫不到；
  cargo-generate 会把 `.liquid` 渲染掉并去掉后缀，生成出来的项目照旧。

## [0.8.1] - 2026-11-17

三处「文档写了、代码没做到」的收口，外加一个 API 缺口。全部向后兼容。

### Added

- **`base::scrollbar`**：覆盖式滚动条的驱动。`base.css` 里 `.c-scroller` / `.c-overlay-scroll` 的样式从 0.8.0 起就在，注释也写着「由 scrollbar.js 接管」—— 但那个脚本从来没被写出来，样式是死的。现在由 Rust 接管：原生宽度归零、拇指自绘、指针进入容器即浮现、可拖动、内容变高自动重算、组件卸载时摘监听与定时器。
- **`ScrollArea` / `NavColumn` 自动挂覆盖式滚动条**，使用方不用额外调用。
- **`Row` 新增 `wide` prop**：控件槽吃掉剩余宽度。滑块、输入框这类长控件放进默认槽（`flex: none`）会被压成一条。
- **`glass-design-host`：`HostOptions` / `install_with`**：可以关掉 `WDA_EXCLUDEFROMCAPTURE`。

### Fixed

- **滚动条拇指必须被精确压住才看得见**：`*:hover::-webkit-scrollbar-thumb` 在 Chromium 里只解析成「拇指自身被悬停」，指针在内容区或轨道空白处都不触发。
- **滚动条占 16px 布局宽度**：内容列被挤窄 16px，左右内边距对不上。改用覆盖式后 `offsetWidth == clientWidth`。
- **滑块拖拽一顿一顿**：`.c-slider__thumb` 的 `transform` 同时承载「按下缩放」和「拖拽位移」，那条 200ms 过渡把位移也一起缓动了 —— 拖拽时 `--p` 每帧都变，等于每帧重起一次过渡，拇指永远追不上指针。位移改走 `translate` 属性、缩放留在 `transform`，两者独立。
- **`install()` 让应用拍不到**：`WDA_EXCLUDEFROMCAPTURE` 的语义是「这个窗口不被任何截屏接口拍到」，系统截图 / 录屏 / 任务栏预览一并失效。默认行为不变，新增 `install_with` 供需要被截图的应用关掉。

### 已知限制

- 覆盖式滚动条依赖 `translate` 属性（Chromium 104+ / Safari 14.1+）。更老的引擎会退回原位、不动 —— 但那时原生滚动条也被 `scrollbar-width: none` 关掉了，等于没有滚动条。库只承诺 WebView2，这条在承诺范围内。

## [0.8.0] - 2026-11-17

从「拷一个文件夹」变成「加一行依赖」。这一版的重点不是新控件，是让库能被 `cargo add` 进来、被 CI 验证、被 `cargo-generate` 起项目。

### Added

- **`bootstrap` / `bootstrap_with` / `BootstrapOptions` / `GlassApp`**（`base/bootstrap.rs`）：把 `install()` + `Theme::provide()` + `OverlayHost` + `mount_to_body` 收成一次调用。手写根节点时用 `GlassApp` 或拆开三步。
- **`InstallConfig` / `install_with` / `styles_css()` / `glass_defs_svg()`**（`base/install.rs`）：注入可以关；样式与贴图内容可以从库外拿到，供构建期落盘。
- **`IconName::custom(markup: &'static str)`**：24 视框的 path 片段，编译期固定，内置 97 个之外不再需要改枚举。
- **`Theme::set_accent` / `Theme::set_token` / `Theme::remove_token`**：强调色有专口（`apply()` 会重写 `--accent*`）；`set_token` 覆盖任意 `--*` 令牌，撤掉即回到 `tokens.css`。
- **10 个布局原语**（`layout/primitives.rs`）：`Stack` / `Cluster` / `Grid` / `Page` / `PageInset` / `Stage` / `Center` / `Push` / `Fill` / `Pad`。页面里不再写裸 `display:flex` / `gap` / `padding`。
- **`glass-design-host` crate**（`host/`）：窗口装配、桌面截屏、壁纸接线从 `应用/src-tauri` 拆出，可被别的宿主复用。依赖 tauri + windows，不依赖 `控件/`。
- **`examples/hello.rs`**：最小可运行示例，演示 `bootstrap` 与 `AppShell` 页面。
- **CI**（`.github/workflows/ci.yml`）：单测、wasm 编译、无默认 feature 编译、发布包文件数断言。
- **`docs/04-作为库分发.md`**、**`template/`**（cargo-generate 模板）。

### Changed

- **依赖从精确锁改成 caret**：`leptos = "0.8"`、`wasm-bindgen = "0.2"`（原来是 `=0.8.23` / `=0.2.130`）。精确锁会让使用方出现两份 crate，`Signal` 变成两个不兼容类型。
- **重新导出 `leptos` 与 `wasm-bindgen`**：给 props 类型与文档一个版本锚点。宏展开是绝对路径，使用方仍需同时声明这两个依赖（见 `控件/README.md` §1）。
- **包体积裁剪**：发布包从全仓库 150 个文件 / 1.6 MiB 收到 84 个文件 / 1.3 MiB（压缩 405.5 KiB）。`include` 白名单只留 `src/**` + `css/glass.css` + `assets/glass-defs.svg` + `examples/**` + `README.md`，`css/src/` 的 64 个编辑分区不进包。
- **组件数口径 90 → 104**：按 `grep -rho "#\[component\]" 控件/src | wc -l` 实测 106 处，扣掉 2 个 `pub(crate)` / 私有内部件后为 104。旧数字 90 是估算值，没有按 `pub` 过滤过。
- **`控件/README.md` 重写**：从“拷文件夹”改成“声明依赖”，四种接入方式、两层入口、`embedded` 开关的收益与代价。
- 根 `README.md`、`docs/02`、`docs/03` 同步组件数、目录结构（新增 `host/`、`template/`）与 crate 体积。

### Fixed

- **首帧无样式**：默认 `embedded` 会把 331 KB CSS 与 134 KB SVG 编进 wasm，首帧必须等 wasm 下载与实例化。关掉 `embedded` 改用 `<link>` 可解决（wasm 同时少 465 KB），代价是多两个静态文件。

### 已知限制

- leptos 版本由使用方声明；应用侧若写 `=0.8.x` 精确锁，会与库的 caret 各拿一份，`Signal` 不兼容。
- `stylesheet` 与 `<div id="gd-glass-defs">` 的去重按 DOM id 判断。两份 `glass-design` 0.x 同时存在时，后挂载的那份会静默复用旧样式（见 `docs/04` §4）。
- 23 个组件文件里还有 87 处兜底中文没接 `t()`。
- 日期值格式仍是 `"YYYY-MM-DD"`，只有面板标题与周表头跟语言走。

## [0.7.0] - 2026-11-15

P4 收官：`DataTable` / `Table` / `CollectionView` 数据展示三件套、变高行、日期区间（`DateRange` / `TimeRange`，可跨零点）、i18n 文案切换。

[0.8.0]: #080---2026-11-17
[0.7.0]: #070---2026-11-15
