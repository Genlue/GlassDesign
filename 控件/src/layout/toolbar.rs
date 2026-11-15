use leptos::prelude::*;

/// 窗口顶栏。空白处可拖窗；内部交互区标 `data-no-drag`。
#[component]
pub fn Toolbar(children: Children) -> impl IntoView {
    view! {
        <div class="c-topbar" data-tauri-drag-region="true">
            {children()}
        </div>
    }
}

#[component]
pub fn ToolbarLead(children: Children) -> impl IntoView {
    view! {
        <div class="c-topbar__lead" data-no-drag="true">{children()}</div>
    }
}

#[component]
pub fn ToolbarPills(children: Children) -> impl IntoView {
    view! {
        <div class="c-topbar__pills" data-no-drag="true">{children()}</div>
    }
}

#[component]
pub fn ToolbarTitle(
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] subtitle: String,
) -> impl IntoView {
    view! {
        <div class="l-row l-row--3" style="min-width:0">
            <span class="c-title">{title}</span>
            {(!subtitle.is_empty()).then(|| view! { <span class="c-subtitle">{subtitle}</span> })}
        </div>
    }
}

/// 胶囊尺寸档位。档位对应构建期生成的一套固定形状贴图；
/// `Auto` 不套 SVG filter，走纯 CSS 兜底（宽度不固定，贴图无法对齐）。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum CapsuleSize {
    #[default]
    Auto,
    /// 112 × 34
    Sm,
    /// 180 × 34
    Md,
    /// 34 × 34 正圆
    Round,
}

/// 一组控件共用的玻璃胶囊。
#[component]
pub fn Capsule(
    children: Children,
    #[prop(optional)] size: CapsuleSize,
) -> impl IntoView {
    let (cls, width) = match size {
        CapsuleSize::Auto => ("c-capsule", None),
        CapsuleSize::Sm => ("c-capsule", Some("112")),
        CapsuleSize::Md => ("c-capsule", Some("180")),
        CapsuleSize::Round => ("c-capsule c-capsule--round", None),
    };
    view! {
        <div class=cls data-size=width data-glass="capsule">{children()}</div>
    }
}
