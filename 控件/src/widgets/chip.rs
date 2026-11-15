use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::base::types::Size;

#[component]
pub fn Chip(
    #[prop(into, optional)] label: String,
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into, optional)] selected: MaybeProp<bool>,
    #[prop(optional)] removable: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(optional)] on_remove: Option<Callback<()>>,
    #[prop(optional)] size: Size,
) -> impl IntoView {
    let interact = on_click.is_some() || removable;
    let mut cls = String::from("c-chip");
    if let Some(s) = size.class() {
        cls.push_str(" c-chip--");
        cls.push_str(s);
    }
    if interact {
        cls.push_str(" c-chip--interactive");
    }

    let pressed = move || {
        if selected.get().unwrap_or(false) {
            "true"
        } else {
            "false"
        }
    };
    let label_for_remove = label.clone();

    view! {
        <div class=cls aria-pressed=pressed>
            {if on_click.is_some() {
                view! {
                    <button class="c-chip__label" type="button" aria-pressed=pressed
                        on:click=move |_| { if let Some(cb) = on_click { cb.run(()); } }>
                        {icon.map(|name| view! { <Icon name=name size=12 /> })}
                        {label.clone()}
                    </button>
                }.into_any()
            } else {
                view! {
                    <span class="c-chip__label">
                        {icon.map(|name| view! { <Icon name=name size=12 /> })}
                        {label.clone()}
                    </span>
                }.into_any()
            }}
            {removable.then(|| view! {
                <button
                    class="c-chip__x"
                    type="button"
                    aria-label=format!("移除 {label_for_remove}")
                    title=format!("移除 {label_for_remove}")
                    on:click=move |ev| {
                        ev.stop_propagation();
                        if let Some(cb) = on_remove { cb.run(()); }
                    }
                >
                    <Icon name=IconName::X size=9 />
                </button>
            })}
        </div>
    }
}
