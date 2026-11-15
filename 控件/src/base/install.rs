use wasm_bindgen::JsCast;

/// 与 `Cargo.toml` 同步；`install()` 会写到 `<html data-gd-version>`。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 由 `scripts/build-css.mjs` 生成。
pub const STYLES: &str = include_str!("../../css/glass.css");

/// 由 `scripts/build-glass.mjs` 生成：预计算位移/高光贴图 + 静态 SVG filter。
pub const GLASS_DEFS: &str = include_str!("../../assets/glass-defs.svg");

#[derive(Clone, Copy, Debug, Default)]
pub struct InstallReport {
    pub styles: bool,
    pub glass_defs: bool,
    pub version: &'static str,
}

/// 注入样式与静态玻璃定义。幂等；可重复调用。
///
/// 没有 JavaScript、canvas、WebGL：`<style>` 是 CSS，`<svg>` 是构建期生成的静态 filter。
/// 不创建窗口、不读文件、不发网络请求。
pub fn install() -> InstallReport {
    let mut report = InstallReport {
        version: VERSION,
        ..Default::default()
    };

    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return report;
    };

    if let Some(root) = doc.document_element() {
        let _ = root.set_attribute("data-gd-version", VERSION);
    }

    if doc.get_element_by_id("gd-styles").is_none() && !STYLES.is_empty() {
        if let Ok(style) = doc.create_element("style") {
            style.set_id("gd-styles");
            style.set_text_content(Some(STYLES));
            if let Some(head) = doc.head() {
                let _ = head.append_child(&style);
                report.styles = true;
            }
        }
    }

    if doc.get_element_by_id("gd-glass-defs").is_none() && !GLASS_DEFS.is_empty() {
        if let Ok(host) = doc.create_element("div") {
            host.set_id("gd-glass-defs");
            let _ = host.set_attribute(
                "style",
                "position:absolute;width:0;height:0;overflow:hidden;pointer-events:none",
            );
            let _ = host.set_attribute("aria-hidden", "true");
            host.set_inner_html(GLASS_DEFS);
            if let Some(body) = doc.body() {
                let _ = body.unchecked_into::<web_sys::Node>().append_child(&host);
                report.glass_defs = true;
            }
        }
    }

    report
}
