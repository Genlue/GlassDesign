use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Crumb {
    pub label: String,
    pub icon: Option<IconName>,
}

impl Crumb {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
        }
    }
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

#[component]
pub fn Breadcrumb(
    #[prop(into)] items: Vec<Crumb>,
    #[prop(optional)] on_pick: Option<Callback<usize>>,
) -> impl IntoView {
    let last = items.len().saturating_sub(1);
    let parts = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let is_last = i == last;
            let on_pick = on_pick;
            let label = item.label.clone();
            view! {
                {(i > 0).then(|| view! {
                    <span class="c-crumbs__sep"><Icon name=IconName::CHEVRON_RIGHT size=10 /></span>
                })}
                {if is_last {
                    view! {
                        <span class="c-crumbs__item" aria-current="page">
                            {item.icon.map(|name| view! { <Icon name=name size=12 /> })}
                            {label}
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <button class="c-crumbs__item" type="button"
                            on:click=move |_| { if let Some(cb) = on_pick { cb.run(i); } }>
                            {item.icon.map(|name| view! { <Icon name=name size=12 /> })}
                            {label}
                        </button>
                    }.into_any()
                }}
            }
        })
        .collect::<Vec<_>>();

    view! {
        <nav class="c-crumbs" aria-label="位置">{parts}</nav>
    }
}
