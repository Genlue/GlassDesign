# 变更记录

本文件格式参照 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

规则：新增 prop / 组件 = minor；改 props 形状、删组件、改 DOM 或键盘契约、改贴图尺寸 = major；CSS 变量改名先加别名再删。

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
