use leptos::prelude::*;

/// 面板玻璃的固定尺寸档。
///
/// 贴图尺寸必须与元素边框盒逐像素一致（`feImage` 不会按元素框拉伸），
/// 所以这里只收「调用方写死尺寸」的三种常用面板；内容高度不定时不要用档位，
/// 直接 `<div class="c-glass-panel">` 走 Auto 的 CSS 涂层。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlassPanelSize {
    /// 320 × 220
    P320x220,
    /// 480 × 320
    P480x320,
    /// 640 × 420
    P640x420,
}

impl GlassPanelSize {
    pub fn glass_id(self) -> &'static str {
        match self {
            Self::P320x220 => "panel-320x220",
            Self::P480x320 => "panel-480x320",
            Self::P640x420 => "panel-640x420",
        }
    }
}

/// 一块面板玻璃。`size=None` 时是普通 `.c-glass-panel`（Auto，无折射）。
#[component]
pub fn GlassPanel(
    #[prop(optional)] size: Option<GlassPanelSize>,
    children: Children,
) -> impl IntoView {
    let glass = size.map(GlassPanelSize::glass_id);
    view! {
        <div class="c-glass-panel" data-glass=glass>{children()}</div>
    }
}
