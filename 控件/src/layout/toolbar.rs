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

/// 顶栏最左的窗口区（红绿灯 + 可选的导航开关）。
///
/// 它必须是 `.c-topbar` 的**第一个 flex 子项**，不是另一层绝对定位：
/// 后面标题组的位移账本（`--chrome-end`）就是按它的宽度算出来的。
/// 少了这一块，标题会平白右移「三个灯的宽度 + 一个 gap」。
#[component]
pub fn ToolbarChrome(children: Children) -> impl IntoView {
    view! {
        <div class="l-chrome" data-no-drag="true">{children()}</div>
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
    /// 额外类名。布局需要按位置挑中某枚胶囊时用（例如 `.l-chrome__toggle`
    /// 要靠 `:has()` 被顶到导航栏右上角），不要用它改材质。
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let (cls, width) = match size {
        CapsuleSize::Auto => ("c-capsule", None),
        CapsuleSize::Sm => ("c-capsule", Some("112")),
        CapsuleSize::Md => ("c-capsule", Some("180")),
        CapsuleSize::Round => ("c-capsule c-capsule--round", None),
    };
    let cls = if class.is_empty() {
        cls.to_string()
    } else {
        format!("{cls} {class}")
    };
    view! {
        <div class=cls data-size=width data-glass="capsule">{children()}</div>
    }
}
