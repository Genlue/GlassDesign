use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::base::id::next_id;

#[component]
pub fn Disclosure(
    #[prop(into)] title: String,
    #[prop(into, optional)] hint: String,
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(optional)] on_toggle: Option<Callback<bool>>,
    children: Children,
) -> impl IntoView {
    let body_id = next_id("gd-disclosure");
    let head_id = format!("{body_id}-head");
    let head_label = head_id.clone();
    let is_open = move || open.get().unwrap_or(false);

    view! {
        <div class="c-disclosure" data-open=move || if is_open() { "1" } else { "0" }>
            <button
                class="c-disclosure__head"
                type="button"
                id=head_id
                aria-expanded=move || is_open().to_string()
                aria-controls=body_id.clone()
                on:click=move |_| {
                    if let Some(cb) = on_toggle { cb.run(!is_open()); }
                }
            >
                <span class="c-disclosure__chev">
                    <Icon name=IconName::CHEVRON_RIGHT size=12 />
                </span>
                <span class="l-fill">{title}</span>
                {(!hint.is_empty()).then(|| view! { <span class="c-disclosure__hint">{hint}</span> })}
            </button>
            <div
                class="c-disclosure__body"
                id=body_id
                role="region"
                aria-labelledby=head_label
                aria-hidden=move || (!is_open()).to_string()
                prop:inert=move || !is_open()
            >
                <div class="c-disclosure__inner">
                    <div class="c-disclosure__content">{children()}</div>
                </div>
            </div>
        </div>
    }
}
