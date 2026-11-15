//! 集合：网格 / 列表两种排布，共用一套选择与键盘模型。
//!
//! 与旧库的差别：旧库用「spacer + 绝对定位」做窗口化，Rust 侧拿不到同步测量，
//! 硬做会变成两阶段渲染 + 一帧延迟。这里先做**非虚拟**的静态排布（CSS Grid），
//! 把窗口化留给真正需要万级数据时再加；`virtual_row_height` 因此没有实现，
//! 见交付报告「简化边界」。
//!
//! `data-static` 是本批新增的状态标记，配合 45-collection.css 末尾追加的
//! 规则把 .c-collection__item 从绝对定位切回正常流。

use leptos::html;
use leptos::prelude::*;

use crate::widgets::selection::{SelectionMode, SelectionState};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum CollectionLayout {
    Grid,
    #[default]
    List,
}

impl CollectionLayout {
    fn class(self) -> &'static str {
        match self {
            CollectionLayout::Grid => "c-collection c-collection--grid",
            CollectionLayout::List => "c-collection c-collection--list",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct CollectionItem {
    pub id: String,
    pub label: String,
    pub subtitle: Option<String>,
    /// 初始选中。后续状态由组件内部维护，回调把完整选中集交回调用方。
    pub selected: bool,
}

impl CollectionItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            subtitle: None,
            selected: false,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }
}

#[component]
pub fn CollectionView(
    #[prop(into)] items: Vec<CollectionItem>,
    #[prop(optional)] layout: CollectionLayout,
    #[prop(default = 3)] columns: u32,
    #[prop(default = SelectionMode::Single)] selection: SelectionMode,
    #[prop(optional)] on_selection_change: Option<Callback<Vec<String>>>,
    #[prop(optional)] on_activate: Option<Callback<String>>,
    /// 列表行的最小高度（px）。网格单元高度由内容决定，不用这个值。
    #[prop(default = 32.0)] item_height: f64,
    #[prop(into, optional)] empty_text: String,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let label = if label.is_empty() {
        "集合".to_string()
    } else {
        label
    };
    let empty_text = if empty_text.is_empty() {
        "没有内容".to_string()
    } else {
        empty_text
    };
    let cols = if layout == CollectionLayout::Grid {
        columns.max(1) as usize
    } else {
        1
    };

    let root_ref = NodeRef::<html::Div>::new();
    let refs: Vec<NodeRef<html::Div>> = (0..items.len()).map(|_| NodeRef::new()).collect();
    let item_count = items.len();

    let order: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
    let initial: Vec<String> = items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.id.clone())
        .collect();
    let sel = RwSignal::new(SelectionState::with_selected(initial));
    let focused = RwSignal::new(if item_count == 0 { None } else { Some(0usize) });

    let refs_focus = refs.clone();
    let move_focus = move |next: usize| {
        if item_count == 0 {
            return;
        }
        let next = next.min(item_count - 1);
        focused.set(Some(next));
        if let Some(el) = refs_focus[next].get_untracked() {
            let _ = el.focus();
        }
    };

    let order_keys = order.clone();
    let order_emit = order.clone();
    let keydown = move |ev: web_sys::KeyboardEvent| {
        if item_count == 0 {
            return;
        }
        let current = focused.get_untracked().unwrap_or(0).min(item_count - 1);
        let last = item_count - 1;
        let mut handled = true;
        let mut next = current;
        match ev.key().as_str() {
            "ArrowRight" => next = (current + 1).min(last),
            "ArrowLeft" => next = current.saturating_sub(1),
            "ArrowDown" => next = (current + cols).min(last),
            "ArrowUp" => next = current.saturating_sub(cols),
            "Home" => next = 0,
            "End" => next = last,
            " " => {
                let id = order_keys[current].clone();
                let mut state = sel.get_untracked();
                let changed = state.click(selection, &id, &order_keys, true, false);
                if changed {
                    sel.set(state);
                    if let Some(cb) = on_selection_change {
                        cb.run(sel.get_untracked().ordered(&order_emit));
                    }
                }
            }
            "Enter" => {
                if let Some(cb) = on_activate {
                    cb.run(order_keys[current].clone());
                }
            }
            "a" | "A" if selection.is_multiple() => {
                let mut state = sel.get_untracked();
                if state.select_all(&order_keys) {
                    sel.set(state);
                    if let Some(cb) = on_selection_change {
                        cb.run(sel.get_untracked().ordered(&order_emit));
                    }
                }
            }
            _ => handled = false,
        }
        if !handled {
            return;
        }
        ev.prevent_default();
        if next != current {
            move_focus(next);
        }
    };

    let item_views = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let id_aria = item.id.clone();
            let id_click = item.id.clone();
            let id_key = item.id.clone();
            let id_activate = item.id.clone();
            let row_ref = refs[i];
            let refs_click = refs.clone();
            let label_text = item.label.clone();
            let subtitle = item.subtitle.clone();
            let order_click = order.clone();
            let order_emit = order.clone();
            view! {
                <div
                    class="c-collection__item"
                    role="option"
                    data-key=id_key
                    aria-selected=move || sel.get().contains(&id_aria).to_string()
                    tabindex=move || {
                        if focused.get() == Some(i) { "0" } else { "-1" }
                    }
                    node_ref=row_ref
                    on:click=move |ev: web_sys::MouseEvent| {
                        if !selection.is_selectable() {
                            return;
                        }
                        focused.set(Some(i));
                        let mut state = sel.get_untracked();
                        let changed = state.click(
                            selection,
                            &id_click,
                            &order_click,
                            ev.ctrl_key() || ev.meta_key(),
                            ev.shift_key(),
                        );
                        if changed {
                            sel.set(state);
                            if let Some(cb) = on_selection_change {
                                cb.run(sel.get_untracked().ordered(&order_emit));
                            }
                        }
                        if let Some(el) = refs_click[i].get_untracked() {
                            let _ = el.focus();
                        }
                    }
                    on:dblclick=move |_| {
                        if let Some(cb) = on_activate {
                            cb.run(id_activate.clone());
                        }
                    }
                >
                    <div class="c-list__body">
                        <div class="c-list__title">{label_text}</div>
                        {subtitle
                            .filter(|s| !s.is_empty())
                            .map(|s| view! { <div class="c-list__subtitle">{s}</div> })}
                    </div>
                </div>
            }
        })
        .collect::<Vec<_>>();

    let style = format!(
        "--cols:{};--item-h:{}px",
        cols,
        if item_height > 0.0 { item_height } else { 32.0 }
    );

    let empty = item_count == 0;
    view! {
        <div
            class=layout.class()
            role="listbox"
            aria-label=label
            aria-multiselectable=selection.is_multiple().then_some("true")
            data-static="1"
            style=style
            node_ref=root_ref
            on:keydown=keydown
        >
            {if empty {
                view! { <div class="c-collection__empty">{empty_text}</div> }.into_any()
            } else {
                view! {
                    <div class="c-collection__spacer">{item_views}</div>
                }
                .into_any()
            }}
        </div>
    }
}
