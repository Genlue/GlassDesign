use wasm_bindgen::JsCast;

/// 与 `Cargo.toml` 同步；`install()` 会写到 `<html data-gd-version>`。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 合并后的样式表，由 `scripts/build-css.mjs` 生成。
///
/// 关掉 `embedded` feature 后这里是**空串**：样式不再进 wasm，
/// 使用方必须自己 `<link rel="stylesheet" href="glass.css">`，否则页面是裸 HTML。
/// 打包脚本要落盘时用 [`styles_css`] 取同一份内容。
#[cfg(feature = "embedded")]
pub const STYLES: &str = include_str!("../../css/glass.css");
#[cfg(not(feature = "embedded"))]
pub const STYLES: &str = "";

/// 静态玻璃定义，由 `scripts/build-glass.mjs` 生成：预计算位移/高光贴图 + 静态 SVG filter。
///
/// 与 [`STYLES`] 一样按 feature 分流：关掉 `embedded` 后是空串，
/// 使用方得自己把 `glass-defs.svg` 插进 `<body>`，否则玻璃退化成纯色。
#[cfg(feature = "embedded")]
pub const GLASS_DEFS: &str = include_str!("../../assets/glass-defs.svg");
#[cfg(not(feature = "embedded"))]
pub const GLASS_DEFS: &str = "";

/// 打包期落盘用。运行期不需要它 —— 常量本身就够。
pub fn styles_css() -> &'static str {
    STYLES
}

/// 打包期落盘用。理由同上。
pub fn glass_defs_svg() -> &'static str {
    GLASS_DEFS
}

/// 注入内容的选择。两个开关独立：只关一个也有意义
/// （例如样式走 `<link>`、玻璃定义仍由 wasm 注入）。
#[derive(Clone, Copy, Debug)]
pub struct InstallConfig {
    pub styles: bool,
    pub glass_defs: bool,
}

/// 默认两个都注入 —— 与 `install()` 的老行为一致。
impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            styles: true,
            glass_defs: true,
        }
    }
}

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
    install_with(InstallConfig::default())
}

/// 按 [`InstallConfig`] 注入。关掉的项连判断都不做，也不动 DOM。
///
/// 幂等与判重逻辑与 `install()` 完全同一份：按 `id` 找，找到就不再插。
pub fn install_with(cfg: InstallConfig) -> InstallReport {
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

    if cfg.styles && doc.get_element_by_id("gd-styles").is_none() && !STYLES.is_empty() {
        if let Ok(style) = doc.create_element("style") {
            style.set_id("gd-styles");
            style.set_text_content(Some(STYLES));
            if let Some(head) = doc.head() {
                let _ = head.append_child(&style);
                report.styles = true;
            }
        }
    }

    if cfg.glass_defs && doc.get_element_by_id("gd-glass-defs").is_none() && !GLASS_DEFS.is_empty() {
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
