// 宿主入口。窗口不透明、无边框：液态玻璃折的是页面底层的合成渐变 #wallpaper，
// 不需要透明窗口，也不做桌面截屏或 WDA_EXCLUDEFROMCAPTURE。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        // 注册兼容宿主命令（`handlers!` 保留，避免破坏现有接线）。
        .invoke_handler(glass_design_host::handlers!())
        .setup(|app| {
            // 宿主初始化（保留兼容性；不再挂 WDA_EXCLUDEFROMCAPTURE）。
            glass_design_host::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动失败");
}
