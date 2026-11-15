use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ListItem {
    pub key: String,
    pub title: String,
    pub subtitle: String,
    pub meta: String,
    pub icon: Option<IconName>,
    pub chevron: bool,
}

impl ListItem {
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            subtitle: String::new(),
            meta: String::new(),
            icon: None,
            chevron: false,
        }
    }
    pub fn subtitle(mut self, v: impl Into<String>) -> Self {
        self.subtitle = v.into();
        self
    }
    pub fn meta(mut self, v: impl Into<String>) -> Self {
        self.meta = v.into();
        self
    }
    pub fn icon(mut self, v: IconName) -> Self {
        self.icon = Some(v);
        self
    }
    pub fn chevron(mut self) -> Self {
        self.chevron = true;
        self
    }
}

#[component]
pub fn List(
    #[prop(into)] items: Vec<ListItem>,
    #[prop(into, optional)] selected: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] on_activate: Option<Callback<String>>,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let refs: Vec<NodeRef<html::Div>> = (0..items.len()).map(|_| NodeRef::new()).collect();
    let all = items.clone();
    let none_selected = move || selected.get().is_none();
    let label = if label.is_empty() {
        "列表".to_string()
    } else {
        label
    };

    let rows = items.into_iter().enumerate().map(|(i, item)| {
        let key_state = item.key.clone();
        let key_tab = key_state.clone();
        let key_click = item.key.clone();
        let key_activate = item.key.clone();
        let key_data = item.key.clone();
        let row_ref = refs[i];
        let all = all.clone();
        let refs = refs.clone();
        let is_first = i == 0;
        let title = item.title.clone();
        let subtitle = item.subtitle.clone();
        let meta = item.meta.clone();
        let on_select = on_select;
        let on_activate = on_activate;
        view! {
            <div
                class=if on_select.is_some() { "c-list__row c-list__row--interactive" } else { "c-list__row" }
                role="option"
                data-key=key_data
                aria-selected=move || if selected.get().as_deref() == Some(key_state.as_str()) { "true" } else { "false" }
                tabindex=move || if selected.get().as_deref() == Some(key_tab.as_str()) || (none_selected() && is_first) { "0" } else { "-1" }
                node_ref=row_ref
                on:click=move |_| {
                    if let Some(cb) = on_select { cb.run(key_click.clone()); }
                }
                on:dblclick=move |_| {
                    if let Some(cb) = on_activate { cb.run(key_activate.clone()); }
                }
                on:keydown={
                    let all = all.clone();
                    let refs = refs.clone();
                    let key = item.key.clone();
                    move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            if let Some(cb) = on_activate { cb.run(key.clone()); }
                            return;
                        }
                        if ev.key() == " " {
                            ev.prevent_default();
                            if let Some(cb) = on_select { cb.run(key.clone()); }
                            return;
                        }
                        let next = match ev.key().as_str() {
                            "ArrowDown" => Some((i + 1).min(all.len().saturating_sub(1))),
                            "ArrowUp" => Some(i.saturating_sub(1)),
                            "Home" => Some(0),
                            "End" => Some(all.len().saturating_sub(1)),
                            _ => None,
                        };
                        if let Some(n) = next {
                            ev.prevent_default();
                            if let Some(el) = refs[n].get() {
                                let _ = el.focus();
                            }
                            if let Some(cb) = on_select { cb.run(all[n].key.clone()); }
                        }
                    }
                }
            >
                {item.icon.map(|name| view! {
                    <div class="c-list__leading"><span class="c-list__icon"><Icon name=name size=16 /></span></div>
                })}
                <div class="c-list__body">
                    <div class="c-list__title">{title}</div>
                    {(!subtitle.is_empty()).then(|| view! { <div class="c-list__subtitle">{subtitle}</div> })}
                </div>
                {(!meta.is_empty()).then(|| view! { <div class="c-list__meta">{meta}</div> })}
                {item.chevron.then(|| view! {
                    <div class="c-list__trailing"><Icon name=IconName::CHEVRON_RIGHT size=13 /></div>
                })}
            </div>
        }
    }).collect::<Vec<_>>();

    view! {
        <div class="c-list" role="listbox" aria-label=label>{rows}</div>
    }
}
