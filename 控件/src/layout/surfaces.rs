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
#[component]
pub fn Row(
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] hint: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="c-row">
            <div class="c-row__label">
                {label}
                {(!hint.is_empty()).then(|| view! { <span class="c-row__hint">{hint}</span> })}
            </div>
            <div class="c-row__control">{children()}</div>
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
