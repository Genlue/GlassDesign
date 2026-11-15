use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TabItem {
    pub value: String,
    pub label: String,
    pub icon: Option<IconName>,
    pub disabled: bool,
}

impl TabItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), icon: None, disabled: false }
    }
    pub fn icon(mut self, icon: IconName) -> Self { self.icon = Some(icon); self }
    pub fn disabled(mut self) -> Self { self.disabled = true; self }
}

#[component]
pub fn Tabs(
    #[prop(into)] tabs: Vec<TabItem>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] closable: bool,
    #[prop(optional)] on_close: Option<Callback<String>>,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let refs: Vec<NodeRef<html::Div>> = (0..tabs.len()).map(|_| NodeRef::new()).collect();
    let all = tabs.clone();
    let label = if label.is_empty() { "页签".to_string() } else { label };

    let items = tabs.into_iter().enumerate().map(|(i, tab)| {
        let state = tab.value.clone();
        let tabindex_value = tab.value.clone();
        let click_value = tab.value.clone();
        let close_value = tab.value.clone();
        let tab_ref = refs[i];
        let all = all.clone();
        let refs = refs.clone();
        let disabled = tab.disabled;
        let label_for_close = tab.label.clone();
        view! {
            <div
                class="c-tabs__tab"
                role="tab"
                data-value=tab.value.clone()
                aria-selected=move || if value.get().as_deref() == Some(state.as_str()) { "true" } else { "false" }
                aria-disabled=disabled.then_some("true")
                tabindex=move || if value.get().as_deref() == Some(tabindex_value.as_str()) { "0" } else { "-1" }
                node_ref=tab_ref
                on:click=move |_| {
                    if disabled { return; }
                    if let Some(cb) = on_change { cb.run(click_value.clone()); }
                }
                on:keydown={
                    let all = all.clone();
                    let refs = refs.clone();
                    let current = tab.value.clone();
                    move |ev: web_sys::KeyboardEvent| {
                        let step = match ev.key().as_str() {
                            "ArrowRight" => 1isize,
                            "ArrowLeft" => -1isize,
                            "Home" => { if let Some(el) = refs[0].get() { let _ = el.focus(); } if let Some(cb)=on_change { cb.run(all[0].value.clone()); } return; }
                            "End" => { let n=all.len()-1; if let Some(el)=refs[n].get(){let _=el.focus();} if let Some(cb)=on_change{cb.run(all[n].value.clone());} return; }
                            _ => return,
                        };
                        ev.prevent_default();
                        let Some(idx) = all.iter().position(|t| t.value == current) else { return };
                        let mut next = idx;
                        for _ in 0..all.len() {
                            next = (next as isize + step).rem_euclid(all.len() as isize) as usize;
                            if !all[next].disabled { break; }
                        }
                        if let Some(el) = refs[next].get() { let _ = el.focus(); }
                        if let Some(cb) = on_change { cb.run(all[next].value.clone()); }
                    }
                }
            >
                {tab.icon.map(|name| view! { <Icon name=name size=13 /> })}
                {(!tab.label.is_empty()).then(|| view! { <span>{tab.label}</span> })}
                {closable.then(|| view! {
                    <button class="c-tabs__close" type="button" aria-label=format!("关闭 {label_for_close}")
                        on:click=move |ev| {
                            ev.stop_propagation();
                            if let Some(cb) = on_close { cb.run(close_value.clone()); }
                        }>
                        <Icon name=IconName::X size=9 />
                    </button>
                })}
            </div>
        }
    }).collect::<Vec<_>>();

    view! {
        <div class="c-tabs" role="tablist" aria-label=label>
            <div class="c-tabs__scroll">{items}</div>
        </div>
    }
}
