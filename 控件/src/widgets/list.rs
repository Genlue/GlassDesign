use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::widgets::selection::{SelectionMode, SelectionState};

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

/// 列表的选择模式。默认单选，保持画廊原有的「点一条换一条」行为。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ListSelection {
    #[default]
    Single,
    Multiple,
}

impl ListSelection {
    fn mode(self) -> SelectionMode {
        match self {
            ListSelection::Single => SelectionMode::Single,
            ListSelection::Multiple => SelectionMode::Multiple,
        }
    }
}

#[component]
pub fn List(
    #[prop(into)] items: Vec<ListItem>,
    #[prop(into, optional)] selected: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] on_activate: Option<Callback<String>>,
    /// 多选时额外上报完整选中集；单选模式下不会触发。
    #[prop(optional)] on_selection_change: Option<Callback<Vec<String>>>,
    #[prop(optional)] selection: ListSelection,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let refs: Vec<NodeRef<html::Div>> = (0..items.len()).map(|_| NodeRef::new()).collect();
    let all = items.clone();
    let multi = selection == ListSelection::Multiple;
    let order: Vec<String> = items.iter().map(|i| i.key.clone()).collect();
    let first_key = items.first().map(|i| i.key.clone());

    // 多选状态自己持有；单选沿用调用方的 `selected`，不重复存一份。
    let state = RwSignal::new(SelectionState::with_selected(
        selected.get_untracked().into_iter().collect::<Vec<_>>(),
    ));
    let focus_key = RwSignal::new(selected.get_untracked().or_else(|| first_key.clone()));

    let effective_focus = Memo::new(move |_| focus_key.get().or_else(|| first_key.clone()));

    let is_selected = move |key: &str| {
        if multi {
            state.get().contains(key)
        } else {
            selected.get().as_deref() == Some(key)
        }
    };

    let label = if label.is_empty() {
        "列表".to_string()
    } else {
        label
    };

    let rows = all
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let key_state = item.key.clone();
            let key_tab = item.key.clone();
            let key_click = item.key.clone();
            let key_activate_dbl = item.key.clone();
            let key_activate_key = item.key.clone();
            let key_data = item.key.clone();
            let key_space = item.key.clone();
            let row_ref = refs[i];
            let all = items.clone();
            let refs = refs.clone();
            let order_click = order.clone();
            let order_space = order.clone();
            let is_first = i == 0;
            let title = item.title.clone();
            let subtitle = item.subtitle.clone();
            let meta = item.meta.clone();
            let mode = selection.mode();
            view! {
                <div
                    class=if on_select.is_some() || multi { "c-list__row c-list__row--interactive" } else { "c-list__row" }
                    role="option"
                    data-key=key_data
                    aria-selected=move || is_selected(&key_state).to_string()
                    tabindex=move || {
                        let at = effective_focus.get();
                        if at.as_deref() == Some(key_tab.as_str()) || (at.is_none() && is_first) {
                            "0"
                        } else {
                            "-1"
                        }
                    }
                    node_ref=row_ref
                    on:click=move |ev: web_sys::MouseEvent| {
                        focus_key.set(Some(key_click.clone()));
                        if multi {
                            let mut next = state.get_untracked();
                            let changed = next.click(
                                mode,
                                &key_click,
                                &order_click,
                                ev.ctrl_key() || ev.meta_key(),
                                ev.shift_key(),
                            );
                            if changed {
                                state.set(next);
                                if let Some(cb) = on_selection_change {
                                    cb.run(state.get_untracked().ordered(&order_click));
                                }
                            }
                        }
                        // 单选保持原契约：只回一个 id，由调用方决定选中态。
                        if let Some(cb) = on_select { cb.run(key_click.clone()); }
                    }
                    on:dblclick=move |_| {
                        if let Some(cb) = on_activate { cb.run(key_activate_dbl.clone()); }
                    }
                    on:keydown={
                        let all = all;
                        let refs = refs;
                        move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                if let Some(cb) = on_activate { cb.run(key_activate_key.clone()); }
                                return;
                            }
                            if ev.key() == " " {
                                ev.prevent_default();
                                if multi {
                                    let mut next = state.get_untracked();
                                    if next.click(mode, &key_space, &order_space, true, false) {
                                        state.set(next);
                                        if let Some(cb) = on_selection_change {
                                            cb.run(state.get_untracked().ordered(&order_space));
                                        }
                                    }
                                } else if let Some(cb) = on_select {
                                    cb.run(key_space.clone());
                                }
                                return;
                            }
                            if multi && (ev.key() == "a" || ev.key() == "A")
                                && (ev.ctrl_key() || ev.meta_key())
                            {
                                ev.prevent_default();
                                let mut next = state.get_untracked();
                                if next.select_all(&order_space) {
                                    state.set(next);
                                    if let Some(cb) = on_selection_change {
                                        cb.run(state.get_untracked().ordered(&order_space));
                                    }
                                }
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
                                focus_key.set(Some(all[n].key.clone()));
                                if let Some(el) = refs[n].get() {
                                    let _ = el.focus();
                                }
                                // 多选：方向键只移动焦点，Space 才改选中。
                                if !multi {
                                    if let Some(cb) = on_select { cb.run(all[n].key.clone()); }
                                }
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
        })
        .collect::<Vec<_>>();

    view! {
        <div
            class="c-list"
            role="listbox"
            aria-label=label
            aria-multiselectable=multi.then_some("true")
        >{rows}</div>
    }
}
