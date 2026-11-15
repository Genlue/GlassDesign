use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn Checkbox(
    #[prop(into, optional)] checked: MaybeProp<bool>,
    #[prop(into, optional)] label: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] invalid: bool,
    #[prop(optional)] on_change: Option<Callback<bool>>,
) -> impl IntoView {
    let current = move || checked.get().unwrap_or(false);
    let has_label = !label.is_empty();

    view! {
        <div
            class="c-check-row"
            role="checkbox"
            aria-checked=move || current().to_string()
            aria-label=(!has_label).then(|| "复选框".to_string())
            aria-disabled=disabled.then_some("true")
            aria-invalid=invalid.then_some("true")
            data-disabled=disabled.then_some("1")
            data-invalid=invalid.then_some("1")
            tabindex=if disabled { "-1" } else { "0" }
            on:click=move |_| {
                if disabled {
                    return;
                }
                if let Some(cb) = on_change {
                    cb.run(!current());
                }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if disabled {
                    return;
                }
                if ev.key() == " " || ev.key() == "Enter" {
                    ev.prevent_default();
                    if let Some(cb) = on_change {
                        cb.run(!current());
                    }
                }
            }
        >
            <span class="c-check" aria-hidden="true" aria-checked=move || current().to_string()>
                {move || current().then(|| view! { <Icon name=IconName::CHECK size=11 /> })}
            </span>
            {has_label.then(|| view! {
                <span style="font-size:var(--fs-13)">{label.clone()}</span>
            })}
        </div>
    }
}
