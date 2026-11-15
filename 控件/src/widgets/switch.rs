use leptos::prelude::*;

/// 开关。受控：点击只上报下一个值，视觉由 `checked` 决定。
#[component]
pub fn Switch(
    #[prop(into, optional)] checked: MaybeProp<bool>,
    #[prop(into, optional)] label: String,
    #[prop(optional)] on_change: Option<Callback<bool>>,
) -> impl IntoView {
    let current = move || checked.get().unwrap_or(false);

    view! {
        <button
            class="c-switch"
            type="button"
            role="switch"
            aria-checked=move || if current() { "true" } else { "false" }
            aria-label=label
            on:click=move |_| {
                if let Some(cb) = on_change {
                    cb.run(!current());
                }
            }
        >
            <span class="c-switch__knob"></span>
        </button>
    }
}
