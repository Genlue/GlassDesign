#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        // 注册兼容宿主命令；`handlers!` 保留以兼容既有调用方式。
        .invoke_handler(glass_design_host::handlers!())
        .setup(|app| {
            // 宿主初始化（保留兼容性；不再挂 WDA_EXCLUDEFROMCAPTURE）。
            glass_design_host::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running GlassDesign");
}
