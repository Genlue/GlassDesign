use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn Dialog(
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] description: String,
    #[prop(optional)] width: u32,
    #[prop(default = true)] dismissible: bool,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] actions: Option<Children>,
) -> impl IntoView {
    let dialog_ref = NodeRef::<html::Div>::new();
    let is_open = move || open.get().unwrap_or(false);
    let width = if width == 0 { 300 } else { width };

    Effect::new(move |_| {
        if is_open() {
            if let Some(el) = dialog_ref.get() {
                let _ = el.focus();
            }
        }
    });

    let close = move || {
        if let Some(cb) = on_close {
            cb.run(());
        }
    };
    let has_head = !title.is_empty() || !description.is_empty();

    view! {
        <div
                    class="c-dialog-scrim"
                    style=move || if is_open() { "z-index:500" } else { "display:none" }
                    data-level="1"
                    on:pointerdown=move |ev| {
                        if !dismissible {
                            return;
                        }
                        if ev.target() == ev.current_target() {
                            close();
                        }
                    }
                >
                    <div
                        class="c-dialog c-glass-sheet"
                        role="dialog"
                        aria-modal="true"
                        aria-label=title.clone()
                        data-level="1"
                        style=format!("--dlg-w:{width}px")
                        tabindex="-1"
                        node_ref=dialog_ref
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Escape" && dismissible {
                                ev.prevent_default();
                                close();
                            }
                        }
                    >
                        {has_head.then(|| view! {
                            <div class="c-dialog__head">
                                <span class="c-dialog__icon"><Icon name=IconName::INFO size=17 /></span>
                                <div class="l-fill">
                                    {(!title.is_empty()).then(|| view! { <div class="c-dialog__title">{title.clone()}</div> })}
                                    {(!description.is_empty()).then(|| view! { <div class="c-dialog__desc">{description.clone()}</div> })}
                                </div>
                            </div>
                        })}
                        {children.map(|c| view! { <div class="c-dialog__body">{c()}</div> })}
                        {actions.map(|a| view! { <div class="c-dialog__actions">{a()}</div> })}
                    </div>
        </div>
    }
}
