# glass-design-host

给 [glass-design](../控件) 的 Tauri 2 宿主胶水。

负责在 Tauri 宿主窗口里打开全铺壳层标记（`data-shell="host"`），并保持 `#wallpaper` 使用 `tokens.css` 的内置合成渐变背景（不再进行桌面截屏，也不挂载 `WDA_EXCLUDEFROMCAPTURE`，从而避免部分显卡驱动黑屏，并兼容远程桌面、系统截图、录屏与任务栏预览）。

接入两步：

1. `.invoke_handler(glass_design_host::handlers!())`，并在 `setup` 里 `glass_design_host::install(app.handle())?`（保留向后兼容的无副作用入口）。
2. 把 `WALLPAPER_JS`（`assets/wallpaper.js`）同步到前端 dist，用普通 `<script src="wallpaper.js">` 引入。仓库里用 `scripts/sync-host-assets.mjs`。
