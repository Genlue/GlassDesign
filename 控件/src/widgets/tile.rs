use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn Tile(
    #[prop(into, optional)] media_src: String,
    #[prop(into, optional)] media_alt: String,
    #[prop(optional)] media_icon: Option<IconName>,
    #[prop(into, optional)] badge: String,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] subtitle: String,
    #[prop(into, optional)] selected: MaybeProp<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(optional)] on_double: Option<Callback<()>>,
) -> impl IntoView {
    let has_caption = !title.is_empty() || !subtitle.is_empty();
    view! {
        <div
            class="c-tile"
            role="option"
            aria-selected=move || selected.get().unwrap_or(false).to_string()
            on:click=move |_| { if let Some(cb) = on_click { cb.run(()); } }
            on:dblclick=move |_| { if let Some(cb) = on_double { cb.run(()); } }
        >
            <div class="c-tile__media">
                {if !media_src.is_empty() {
                    view! { <img src=media_src alt=media_alt draggable="false" /> }.into_any()
                } else if let Some(name) = media_icon {
                    view! { <Icon name=name size=28 /> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {(!badge.is_empty()).then(|| view! { <span class="c-tile__badge">{badge}</span> })}
            </div>
            {has_caption.then(|| view! {
                <div class="c-tile__caption">
                    {(!title.is_empty()).then(|| view! { <div class="c-tile__title">{title.clone()}</div> })}
                    {(!subtitle.is_empty()).then(|| view! { <div class="c-tile__subtitle">{subtitle.clone()}</div> })}
                </div>
            })}
        </div>
    }
}
