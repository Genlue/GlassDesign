use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn MediaTile(
    #[prop(into)] src: String,
    #[prop(into, optional)] alt: String,
    #[prop(into, optional)] selected: MaybeProp<bool>,
    #[prop(optional)] multi: bool,
    #[prop(optional)] fav: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(optional)] on_double: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div
            class="c-thumb"
            role="option"
            aria-selected=move || selected.get().unwrap_or(false).to_string()
            data-multi=multi.to_string()
            on:click=move |_| { if let Some(cb) = on_click { cb.run(()); } }
            on:dblclick=move |_| { if let Some(cb) = on_double { cb.run(()); } }
        >
            <img src=src alt=alt draggable="false" />
            <div class="c-thumb__veil"></div>
            <span class="c-thumb__check"><Icon name=IconName::CHECK size=11 /></span>
            {fav.then(|| view! { <span class="c-thumb__fav"><Icon name=IconName::HEART size=13 /></span> })}
        </div>
    }
}
