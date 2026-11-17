// 宿主入口。窗口不透明、无边框：桌面像素由 host crate 截下来画进 #wallpaper，
// 不是靠窗口透明透出来的 —— backdrop-filter 采不到窗口外的像素。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        // 截屏命令由 host crate 提供，`handlers!` 展开成 generate_handler!。
        .invoke_handler(glass_design_host::handlers!())
        .setup(|app| {
            // 给窗口挂 WDA_EXCLUDEFROMCAPTURE，否则截到的「桌面」里包含自己。
            glass_design_host::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动失败");
}
