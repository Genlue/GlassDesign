use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn NavCard(
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] desc: String,
    #[prop(optional)] icon: Option<IconName>,
    #[prop(optional)] solid: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    if solid {
        view! {
            <div
                class="c-navcard c-navcard--solid"
                role="button"
                tabindex="0"
                on:click=move |_| { if let Some(cb) = on_click { cb.run(()); } }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" || ev.key() == " " {
                        ev.prevent_default();
                        if let Some(cb) = on_click { cb.run(()); }
                    }
                }
            >
                <div class="c-navcard__row">
                    {icon.map(|name| view! { <span class="c-navcard__icon"><Icon name=name size=13 /></span> })}
                    <span class="c-navcard__title">{title.clone()}</span>
                    <span class="c-navcard__chev"><Icon name=IconName::CHEVRON_RIGHT size=12 /></span>
                </div>
                {(!desc.is_empty()).then(|| view! { <div class="c-navcard__desc">{desc.clone()}</div> })}
            </div>
        }.into_any()
    } else {
        view! {
            <div class="c-navcard">
                {(!title.is_empty()).then(|| view! { <div class="c-navcard__label">{title.clone()}</div> })}
                {children.map(|c| c())}
            </div>
        }.into_any()
    }
}
