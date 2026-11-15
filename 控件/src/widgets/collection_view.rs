//! 集合：网格 / 列表两种排布，共用一套选择与键盘模型。
//!
//! 排布分两种模式：
//! 1) 默认（小数据集）：`data-static="1"` 把选项切回正常流，交给 CSS Grid 排，
//!    不做任何测量。
//! 2) 虚拟（`virtual_row_height` + `max_height` 都给）：容器自己成为滚动视口，
//!    只渲染窗口内的行，上下两个撑高行补出剩余高度。排布仍是同一套静态 grid
//!    —— 列宽、gap、圆角、选中底色全部沿用非虚拟版本的规则，视觉不会分叉。
//!
//! 虚拟模式下**行高必须是固定值**：`--item-pitch`（行距 = 项高 + gap）由 prop
//! 写进内联样式，CSS 用它反推项高、Rust 用它算窗口，两边只有一个数。
//!
//! 选择与键盘一律按**全量下标**算（`order` / `item_count`），渲染窗口只影响
//! 「DOM 里有哪些项」，不影响状态机。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::base::virtual_window::{self, Window};
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

/// 下一帧再执行。窗口刚变时 DOM 还没落定，焦点搬运要等它之后。
fn schedule(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}

/// 焦点是否还在容器里。窗口滑动会把焦点项移出 DOM，浏览器随即把焦点丢回 body。
fn holds_focus(root: &web_sys::Element) -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .is_some_and(|a| root.contains(Some(a.unchecked_ref::<web_sys::Node>())))
}

/// 焦点掉到 body 时收回容器：虚拟模式下焦点项随时可能被窗口摘掉，
/// 不收回来键盘事件就没人接了。只在「确实落到 body」时动手。
fn reclaim_focus(root: &web_sys::Element) {
    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element());
    if active.is_some_and(|a| a.tag_name() != "BODY") {
        return;
    }
    if let Ok(html) = root.clone().dyn_into::<web_sys::HtmlElement>() {
        let _ = html.focus();
    }
}

/// 聚焦，但不要让浏览器自己滚。
///
/// `focus()` 会顺手把目标滚进视口，而且 Chrome 是**居中**滚：连续按 ↓ 时，
/// 项每碰到下边缘就被拽到视口正中，看起来像一行一行地跳。滚动统一由
/// `ensure_row`（只滚到刚好可见）接管。
fn focus_without_scroll(el: &web_sys::HtmlElement) {
    let Ok(focus) = js_sys::Reflect::get(el.as_ref(), &JsValue::from_str("focus")) else {
        return;
    };
    let Some(focus) = focus.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("preventScroll"), &JsValue::TRUE);
    let _ = focus.call1(el.as_ref(), opts.as_ref());
}

/// 虚拟模式下 NodeRef 可能还指着上一屏已经被摘掉的节点：聚焦前先确认它还在文档里。
fn live(ref_: Option<&NodeRef<html::Div>>) -> Option<web_sys::HtmlElement> {
    let el = ref_?.get_untracked()?;
    el.is_connected().then(|| el.into())
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
    /// 虚拟滚动的固定**行距**（项高 + 间距）。CSS 的 `--item-pitch` 与窗口
    /// 计算共用它；不传就保持静态排布。
    #[prop(optional)] virtual_row_height: Option<f64>,
    /// >0 时组件自己成为滚动视口（`overflow-y:auto; max-height:Npx`）。
    #[prop(default = 0.0)] max_height: f64,
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

    // 两个条件都满足才真正虚拟化：只给行距没有视口，量不出「该渲染哪几行」。
    let pitch = virtual_row_height.unwrap_or(item_height.max(1.0)).max(1.0);
    let virtual_mode = virtual_row_height.is_some() && max_height > 0.0;

    let root_ref = NodeRef::<html::Div>::new();
    let item_count = items.len();
    let refs: Vec<NodeRef<html::Div>> = (0..item_count).map(|_| NodeRef::new()).collect();
    // 窗口以**行**为单位：网格一行 cols 项，列表一行一项。
    let total_rows = item_count.div_ceil(cols);

    let order: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
    let initial: Vec<String> = items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.id.clone())
        .collect();
    let sel = RwSignal::new(SelectionState::with_selected(initial));
    let focused = RwSignal::new(if item_count == 0 { None } else { Some(0usize) });

    /* ------------------------------------------------------------ 虚拟窗口 */

    // 窗口算法在 `base/virtual_window`：与 `DataTable` / `TreeView` 共用一份。
    let win = RwSignal::new(if virtual_mode {
        // 首帧没有滚动事件，先按 max_height 估一个够用的窗口（只会偏大）。
        let visible = (max_height / pitch).ceil() as usize + 2 * virtual_window::OVERSCAN + 1;
        Window::new(0, visible.min(total_rows))
    } else {
        Window::full(total_rows)
    });
    let clamp_win = move || virtual_window::clamp(win.get(), total_rows);

    let all_items = items;
    let visible = Memo::new(move |_| {
        if !virtual_mode {
            return all_items.iter().cloned().enumerate().collect::<Vec<_>>();
        }
        let w = virtual_window::clamp(win.get(), total_rows);
        let start = (w.start() * cols).min(item_count);
        let end = (w.end() * cols).min(item_count);
        all_items[start..end]
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, item)| (start + i, item))
            .collect()
    });

    // max_height 只是上限，实际高度可能更小：挂载后按真实视口再量一次。
    let measured = RwSignal::new(false);
    Effect::new(move |_| {
        if !virtual_mode || measured.get_untracked() {
            return;
        }
        measured.set(true);
        schedule(move || {
            if let Some(el) = root_ref.get_untracked() {
                let next =
                    virtual_window::window_of(&el, pitch, total_rows, virtual_window::OVERSCAN);
                if win.get_untracked() != next {
                    win.set(next);
                }
            }
        });
    });

    // 让第 `row` 行落进窗口：先滚（必要时），再按新的滚动位置重算窗口。
    let ensure_row = move |root: &web_sys::Element, row: usize| {
        let vh = root.client_height() as f64;
        let top = root.scroll_top() as f64;
        let y = row as f64 * pitch;
        let next = if y < top {
            y
        } else if y + pitch > top + vh {
            y + pitch - vh
        } else {
            top
        };
        if next != top {
            root.set_scroll_top(next.max(0.0).round() as i32);
        }
        let next = virtual_window::window_of(root, pitch, total_rows, virtual_window::OVERSCAN);
        if win.get_untracked() != next {
            win.set(next);
        }
    };

    let refs_focus = refs.clone();
    let refs_keys = refs.clone();
    let move_focus = move |next: usize| {
        if item_count == 0 {
            return;
        }
        let next = next.min(item_count - 1);
        focused.set(Some(next));
        let root = root_ref.get_untracked();
        if virtual_mode {
            // 每次移动都过一次「刚好可见」：`focus()` 被关掉了自动滚动，
            // 不补这一步的话，项会在下边缘被切一半。
            if let Some(root) = root.as_ref() {
                ensure_row(root, next / cols);
            }
        }
        match live(refs_focus.get(next)) {
            Some(el) => focus_without_scroll(&el),
            None => {
                // 新窗口还没落到 DOM 上：焦点先挂在容器上（键盘才不会断），
                // 下一帧再交给选项本身。
                if virtual_mode {
                    if let Some(html) = root
                        .clone()
                        .and_then(|r| r.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = html.focus();
                    }
                    let refs_later = refs_focus.clone();
                    schedule(move || {
                        if focused.get_untracked() != Some(next) {
                            return;
                        }
                        if let Some(el) = live(refs_later.get(next)) {
                            focus_without_scroll(&el);
                        }
                    });
                }
            }
        }
    };

    let on_scroll = move |_| {
        if !virtual_mode {
            return;
        }
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        // 焦点项被移出 DOM 时浏览器会把焦点丢回 body：先记下「刚才焦点还在集合里」。
        let had_focus = holds_focus(&root);
        let next = virtual_window::window_of(&root, pitch, total_rows, virtual_window::OVERSCAN);
        if win.get_untracked() != next {
            win.set(next);
        }
        // 焦点项滚出窗口后键盘事件就没人接了：把焦点交给容器，
        // 下一次按键再按 `focused` 里的下标把焦点搬回去。
        // DOM 更新是异步的（项摘掉之后焦点才掉到 body），所以要等下一帧再收。
        if had_focus {
            let root_later = root.clone();
            schedule(move || reclaim_focus(&root_later));
        }
    };

    /* -------------------------------------------------------------- 键盘 */

    let order_keys = order.clone();
    let order_emit = order.clone();
    let keydown = move |ev: web_sys::KeyboardEvent| {
        if item_count == 0 {
            return;
        }
        let current = focused.get_untracked().unwrap_or(0).min(item_count - 1);
        let last = item_count - 1;

        // 用户可能先用滚轮把焦点项滚出了渲染窗口（此时焦点挂在容器上）。
        // 按键要按「焦点项」算，就得先把它拉回视口。
        if virtual_mode {
            let row = current / cols;
            if !clamp_win().contains(row) {
                if let Some(root) = root_ref.get_untracked() {
                    ensure_row(&root, row);
                    if let Some(el) = live(refs_keys.get(current)) {
                        focus_without_scroll(&el);
                    }
                }
            }
        }

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

    /* ---------------------------------------------------------------- 项 */

    let item_view = {
        let sel = sel;
        let focused = focused;
        let order = order.clone();
        move |i: usize, item: CollectionItem| -> AnyView {
            let id_aria = item.id.clone();
            let id_click = item.id.clone();
            let id_key = item.id.clone();
            let id_activate = item.id.clone();
            let row_ref = refs[i];
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
                        if let Some(el) = row_ref.get_untracked() {
                            focus_without_scroll(&el);
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
            .into_any()
        }
    };

    let item_h = if item_height > 0.0 { item_height } else { 32.0 };
    let style = if virtual_mode {
        format!(
            "--cols:{cols};--item-h:{item_h}px;--item-pitch:{pitch}px;--collection-max-h:{max_height}px"
        )
    } else {
        format!("--cols:{cols};--item-h:{item_h}px")
    };

    let empty = item_count == 0;
    view! {
        <div
            class=layout.class()
            role="listbox"
            aria-label=label
            aria-multiselectable=selection.is_multiple().then_some("true")
            data-static="1"
            data-virtual=virtual_mode.then_some("1")
            style=style
            tabindex=virtual_mode.then_some("-1")
            node_ref=root_ref
            on:keydown=keydown
            on:scroll=on_scroll
        >
            {if empty {
                view! { <div class="c-collection__empty">{empty_text}</div> }.into_any()
            } else {
                view! {
                    {virtual_mode.then(|| {
                        view! {
                            <div
                                class="c-collection__pad"
                                role="presentation"
                                aria-hidden="true"
                                data-rows=move || clamp_win().start().to_string()
                                style=move || format!("--rows:{}", clamp_win().start())
                            ></div>
                        }
                    })}
                    <div class="c-collection__spacer">
                        <For
                            each=move || visible.get()
                            key=|(_, item): &(usize, CollectionItem)| item.id.clone()
                            children=move |(i, item): (usize, CollectionItem)| item_view(i, item)
                        />
                    </div>
                    {virtual_mode.then(|| {
                        view! {
                            <div
                                class="c-collection__pad"
                                role="presentation"
                                aria-hidden="true"
                                data-rows=move || {
                                    total_rows.saturating_sub(clamp_win().end()).to_string()
                                }
                                style=move || {
                                    format!(
                                        "--rows:{}",
                                        total_rows.saturating_sub(clamp_win().end()),
                                    )
                                }
                            ></div>
                        }
                    })}
                }
                .into_any()
            }}
        </div>
    }
}
