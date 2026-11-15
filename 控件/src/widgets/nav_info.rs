use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::widgets::avatar::initials;

#[component]
pub fn NavInfo(
    #[prop(into, optional)] avatar: String,
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] subtitle: String,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    let interactive = on_click.is_some();
    let text = initials(&title);
    view! {
        <div
            class=if interactive { "c-navinfo c-navinfo--interactive" } else { "c-navinfo" }
            role=interactive.then_some("button")
            tabindex=interactive.then_some("0")
            on:click=move |_| { if let Some(cb) = on_click { cb.run(()); } }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if !interactive { return; }
                if ev.key() == "Enter" || ev.key() == " " {
                    ev.prevent_default();
                    if let Some(cb) = on_click { cb.run(()); }
                }
            }
        >
            <span class="c-navinfo__media">
                {if !avatar.is_empty() {
                    view! { <img src=avatar alt="" draggable="false" /> }.into_any()
                } else if let Some(name) = icon {
                    view! { <span class="c-navinfo__app"><Icon name=name size=18 /></span> }.into_any()
                } else {
                    view! { <span>{text}</span> }.into_any()
                }}
            </span>
            <span class="c-navinfo__body">
                <span class="c-navinfo__title">{title}</span>
                {(!subtitle.is_empty()).then(|| view! { <span class="c-navinfo__sub">{subtitle}</span> })}
            </span>
        </div>
    }
}
