use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

/// 单行输入。受控：值由调用方持有，`on_input` 只上报用户输入。
#[component]
pub fn Field(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into, optional)] input_type: String,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] disabled: MaybeProp<bool>,
    #[prop(into, optional)] readonly: MaybeProp<bool>,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(optional)] on_enter: Option<Callback<String>>,
) -> impl IntoView {
    let input_type = if input_type.is_empty() {
        "text".to_string()
    } else {
        input_type
    };

    view! {
        <div class="c-field">
            {icon.map(|name| view! {
                <span class="c-field__icon"><Icon name=name size=14 /></span>
            })}
            <input
                class="c-field__input selectable"
                type=input_type
                placeholder=placeholder
                aria-label=label
                value=move || value.get().unwrap_or_default()
                prop:value=move || value.get().unwrap_or_default()
                disabled=move || disabled.get().unwrap_or(false)
                readonly=move || readonly.get().unwrap_or(false)
                on:input=move |ev| {
                    if let Some(cb) = on_input {
                        cb.run(event_target_value(&ev));
                    }
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() != "Enter" || ev.is_composing() || ev.key_code() == 229 {
                        return;
                    }
                    if let Some(cb) = on_enter {
                        cb.run(event_target_value(&ev));
                    }
                }
            />
        </div>
    }
}
