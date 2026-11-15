use std::cell::Cell;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::widgets::inline_message::Tone;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ToastItem {
    pub id: u32,
    pub message: String,
    pub tone: Tone,
    pub icon: Option<IconName>,
}

thread_local! {
    static TOASTS: RwSignal<Vec<ToastItem>> = RwSignal::new(Vec::new());
    static NEXT_ID: Cell<u32> = const { Cell::new(0) };
}

fn icon_for(tone: Tone) -> IconName {
    match tone {
        Tone::Info => IconName::INFO,
        Tone::Success => IconName::CIRCLE_CHECK,
        Tone::Warning => IconName::TRIANGLE_ALERT,
        Tone::Danger => IconName::X_CIRCLE,
    }
}

/// 发一条提示。3.2 秒后自动移除。
pub fn toast(message: impl Into<String>) -> u32 {
    toast_with(message, Tone::Info, None)
}

pub fn toast_with(message: impl Into<String>, tone: Tone, icon: Option<IconName>) -> u32 {
    let id = NEXT_ID.with(|n| {
        let v = n.get() + 1;
        n.set(v);
        v
    });
    let item = ToastItem {
        id,
        message: message.into(),
        tone,
        icon: icon.or_else(|| Some(icon_for(tone))),
    };
    let signal = TOASTS.with(|s| *s);
    signal.update(|list| list.push(item));
    schedule_remove(signal, id);
    id
}

fn schedule_remove(signal: RwSignal<Vec<ToastItem>>, id: u32) {
    let Some(window) = web_sys::window() else { return };
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
        signal.update(|list| list.retain(|t| t.id != id));
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.unchecked_ref::<js_sys::Function>(),
        3200,
    );
}

#[component]
pub fn ToastHost() -> impl IntoView {
    let signal = TOASTS.with(|s| *s);
    view! {
        <div class="c-toast-host" role="status" aria-live="polite" data-layer-ignore="">
            <For each=move || signal.get() key=|t| t.id let:item>
                <div class="c-toast-slot">
                    <div class=format!("c-toast c-toast--{}", match item.tone {
                        Tone::Info => "info",
                        Tone::Success => "success",
                        Tone::Warning => "warning",
                        Tone::Danger => "danger",
                    })>
                        {item.icon.map(|name| view! { <Icon name=name size=14 /> })}
                        {item.message}
                    </div>
                </div>
            </For>
        </div>
    }
}

