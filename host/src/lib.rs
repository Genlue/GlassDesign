//! GlassDesign 的宿主胶水：窗口壳层与壁纸接线。
//!
//! 液态玻璃材质的底层输入 `#wallpaper` 统一使用 CSS 合成渐变背景（`tokens.css` 的
//! `--wallpaper`），不再进行桌面截图（`BitBlt` / `GetDIBits`）或挂载
//! `WDA_EXCLUDEFROMCAPTURE`，从而避免部分显卡驱动下的黑屏问题，并保证系统截图、
//! 录屏、远程桌面与任务栏预览正常工作。
//!
//! 接入只需要两步：
//! 1. `.invoke_handler(glass_design_host::handlers!())` + `glass_design_host::install(app.handle())`
//! 2. 把 [`WALLPAPER_JS`] 同步成 `dist/wallpaper.js`（见 `scripts/sync-host-assets.mjs`）

/// 前端壁纸接线脚本：在 Tauri 宿主环境中写入 `data-shell="host"`，并保持 `#wallpaper`
/// 使用合成渐变背景（`data-source="demo"`）。
///
/// 应用把它落到 dist 里当普通 `<script>` 跑（不是 ESM），页面一行都不用写这段。
pub const WALLPAPER_JS: &str = include_str!("../assets/wallpaper.js");

/// 一行注册命令：`app.invoke_handler(glass_design_host::handlers!())`
///
/// 保留为兼容占位宏（注册兼容命令 [`capture_backdrop`]），下游无需改调用方式。
#[macro_export]
macro_rules! handlers {
    () => {
        tauri::generate_handler![$crate::capture_backdrop]
    };
}

/// 兼容保留命令：桌面背景截屏已永久停用，统一使用 CSS 合成渐变 `#wallpaper`。
///
/// 始终返回 `Err(...)`，不再调用 Win32 GDI 或修改窗口显示亲和性。
mod command {
    #[tauri::command]
    pub fn capture_backdrop(_window: tauri::WebviewWindow) -> Result<String, String> {
        Err("desktop backdrop capture is disabled; using synthetic gradient #wallpaper".into())
    }
}

pub use command::capture_backdrop;

/// 宿主装配的开关（保留用于 API 向后兼容）。
#[derive(Debug, Clone, Copy, Default)]
pub struct HostOptions {
    /// 已停用的兼容字段：不再挂 `WDA_EXCLUDEFROMCAPTURE`，默认值为 `false`。
    pub exclude_from_capture: bool,
}

/// 宿主初始化入口（当前为安全空操作，不再设置 `WDA_EXCLUDEFROMCAPTURE`）。
///
/// 保留签名以兼容现有调用方；窗口可正常被系统截图、录屏、远程桌面与任务栏预览捕获。
pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    install_with(app, HostOptions::default())
}

/// 同 [`install`]，保留用于 API 向后兼容（不再设置 `WDA_EXCLUDEFROMCAPTURE`）。
pub fn install_with(_app: &tauri::AppHandle, _opts: HostOptions) -> Result<(), String> {
    Ok(())
}
