use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

/// 窗口顶栏。空白处与标题可拖窗；内部胶囊与交互控件自动排除。
#[component]
pub fn Toolbar(children: Children) -> impl IntoView {
    view! {
        <div class="c-topbar" data-tauri-drag-region="deep">
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
    #[prop(into, optional)]
    class: String,
) -> impl IntoView {
    let (cls, width, glass) = match size {
        CapsuleSize::Auto => ("c-capsule", None, None),
        CapsuleSize::Sm => ("c-capsule", Some("112"), Some("capsule-112")),
        CapsuleSize::Md => ("c-capsule", Some("180"), Some("capsule-180")),
        CapsuleSize::Round => (
            "c-capsule c-capsule--round c-capsule--circle",
            None,
            Some("capsule-34"),
        ),
    };
    let cls = if class.is_empty() {
        cls.to_string()
    } else {
        format!("{cls} {class}")
    };
    view! {
        <div class=cls data-size=width data-glass=glass data-tauri-drag-region="false">{children()}</div>
    }
}

/// 前进 / 后退双按钮玻璃胶囊（`.c-capsule--nav` + `.c-navpair__btn` + `.c-navpair__sep`）。
///
/// 常见于内容页左上角（`<ToolbarLead>` 内），中间以细线分隔；
/// 禁用态自动降透明度并关闭指针命中。
#[component]
pub fn NavPair(
    #[prop(into, optional)] can_back: MaybeProp<bool>,
    #[prop(into, optional)] can_forward: MaybeProp<bool>,
    #[prop(optional)] on_back: Option<Callback<()>>,
    #[prop(optional)] on_forward: Option<Callback<()>>,
    #[prop(into, optional)] back_label: String,
    #[prop(into, optional)] forward_label: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let back_lbl = if back_label.is_empty() {
        "后退".to_string()
    } else {
        back_label
    };
    let fwd_lbl = if forward_label.is_empty() {
        "前进".to_string()
    } else {
        forward_label
    };
    let cls = if class.is_empty() {
        "c-capsule--nav".to_string()
    } else {
        format!("c-capsule--nav {class}")
    };
    let back_disabled = move || !can_back.get().unwrap_or(false);
    let fwd_disabled = move || !can_forward.get().unwrap_or(false);

    view! {
        <Capsule class=cls>
            <button
                type="button"
                class="c-navpair__btn"
                disabled=back_disabled
                aria-label=back_lbl.clone()
                title=back_lbl
                on:click=move |_| {
                    if let Some(cb) = on_back {
                        cb.run(());
                    }
                }
            >
                <Icon name=IconName::CHEVRON_LEFT size=15 />
            </button>
            <span class="c-navpair__sep" aria-hidden="true"></span>
            <button
                type="button"
                class="c-navpair__btn"
                disabled=fwd_disabled
                aria-label=fwd_lbl.clone()
                title=fwd_lbl
                on:click=move |_| {
                    if let Some(cb) = on_forward {
                        cb.run(());
                    }
                }
            >
                <Icon name=IconName::CHEVRON_RIGHT size=15 />
            </button>
        </Capsule>
    }
}
