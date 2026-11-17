use leptos::prelude::*;

/// 一块卡片表面。只负责结构，不写样式。
#[component]
pub fn Card(children: Children, #[prop(into, optional)] class: String) -> impl IntoView {
    let cls = if class.is_empty() {
        "c-card".to_string()
    } else {
        format!("c-card {class}")
    };
    view! { <section class=cls>{children()}</section> }
}

/// 一行：左侧标签/说明，右侧控件。
///
/// `wide` 让控件槽吃掉剩余宽度。默认槽是 `flex: none`（贴右侧、按内容宽），
/// 滑块、输入框这类需要横向空间的控件放进去会被压成一条 —— 这种行要开 `wide`。
#[component]
pub fn Row(
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] hint: String,
    #[prop(optional)] wide: bool,
    children: Children,
) -> impl IntoView {
    let control = if wide {
        "c-row__control c-row__control--wide"
    } else {
        "c-row__control"
    };
    view! {
        <div class="c-row">
            <div class="c-row__label">
                {label}
                {(!hint.is_empty()).then(|| view! { <span class="c-row__hint">{hint}</span> })}
            </div>
            <div class=control>{children()}</div>
        </div>
    }
}

#[component]
pub fn SectionTitle(children: Children) -> impl IntoView {
    view! { <div class="c-section">{children()}</div> }
}

#[component]
pub fn Divider() -> impl IntoView {
    view! { <div class="c-divider"></div> }
}
