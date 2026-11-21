use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn SearchField(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] width: u32,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] id: String,
    #[prop(into, optional)] disabled: MaybeProp<bool>,
    #[prop(optional)] on_input: Option<Callback<String>>,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();
    let width = if width == 0 { 180 } else { width };
    let placeholder = if placeholder.is_empty() {
        "搜索".to_string()
    } else {
        placeholder
    };
    let aria = if label.is_empty() {
        placeholder.clone()
    } else {
        label
    };
    let id = (!id.is_empty()).then_some(id);
    let is_disabled = move || disabled.get().unwrap_or(false);
    let has_value = move || !value.get().unwrap_or_default().is_empty();
    let focus = move || {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    view! {
        <div class="c-field c-search" style=format!("width:{width}px")>
            <span class="c-field__icon"><Icon name=IconName::SEARCH size=14 /></span>
            <input
                class="c-field__input selectable"
                type="text"
                placeholder=placeholder
                aria-label=aria
                id=id
                disabled=is_disabled
                value=move || value.get().unwrap_or_default()
                prop:value=move || value.get().unwrap_or_default()
                node_ref=input_ref
                on:input=move |ev| {
                    if !is_disabled() && !ev.dyn_ref::<web_sys::InputEvent>().is_some_and(|e| e.is_composing()) {
                        if let Some(cb) = on_input { cb.run(event_target_value(&ev)); }
                    }
                }
                on:compositionend=move |ev| {
                    if !is_disabled() {
                        if let Some(cb) = on_input { cb.run(event_target_value(&ev)); }
                    }
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() != "Escape" || ev.is_composing() || ev.key_code() == 229 || is_disabled() {
                        return;
                    }
                    if let Some(cb) = on_input { cb.run(String::new()); }
                    focus();
                }
            />
            <Show when=has_value>
                <button
                    class="c-iconbtn c-iconbtn--sm"
                    type="button"
                    aria-label="清除"
                    disabled=is_disabled
                    style="color:var(--text-tertiary)"
                    on:click=move |_| {
                        if is_disabled() { return; }
                        if let Some(cb) = on_input { cb.run(String::new()); }
                        focus();
                    }
                >
                    <Icon name=IconName::X_CIRCLE size=14 />
                </button>
            </Show>
        </div>
    }
}
