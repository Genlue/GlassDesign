#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        // 截屏命令由 host crate 提供；`handlers!` 里已经展开成 generate_handler!。
        .invoke_handler(glass_design_host::handlers!())
        .setup(|app| {
            // 挂 WDA_EXCLUDEFROMCAPTURE：否则截到的「桌面」里包含自己（镜中镜）。
            glass_design_host::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running GlassDesign");
}
