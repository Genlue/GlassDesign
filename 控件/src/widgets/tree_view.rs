//! 树：文件树 / 图层 / 大纲。
//!
//! 两条与「递归画 DOM」相反的取舍（沿用旧库结论）：
//! 1) 只渲染可见行：收起的子树根本不在 DOM 里，展开状态用 `HashSet` 管。
//! 2) 扁平 DOM + `aria-level`，不嵌套 `role="group"`：可见行本来就是一维序列，
//!    「下移一行」就是数组下标 +1，键盘逻辑不必进出子树。
//!
//! 展开态、焦点、选中分开存：收起父节点不该丢选中；方向键只移动焦点，
//! Space / 点击才改变选中。
//!
//! 3) 虚拟滚动按「扁平化后的可见行」切片：索引空间本来就是一条一维序列，
//!    窗口只决定 `[start, end)` 这一段进 DOM，上下两个 spacer 撑出剩余高度。
//!    窗口算法在 `base/virtual_window`，与 `DataTable` 共用一份。
//!    代价是「焦点行可能不在 DOM 里」：所有搬运焦点的地方都先滚进窗口再聚焦。

use std::collections::HashSet;

use leptos::attr::custom::custom_attribute;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::base::icon::{Icon, IconName};
use crate::base::virtual_window::{self, Window};

/// 行内过程状态：加载中 / 加载失败。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TreeState {
    Loading,
    Error(String),
}

impl TreeState {
    pub fn error(message: impl Into<String>) -> Self {
        TreeState::Error(message.into())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub icon: Option<IconName>,
    pub badge: Option<String>,
    pub children: Vec<TreeNode>,
    pub disabled: bool,
    /// 行内加载状态。放在节点上而不是全局：失败的是「这一支」，不是整棵树。
    pub state: Option<TreeState>,
}

impl TreeNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            badge: None,
            children: Vec::new(),
            disabled: false,
            state: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn loading(mut self) -> Self {
        self.state = Some(TreeState::Loading);
        self
    }

    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.state = Some(TreeState::Error(message.into()));
        self
    }
}

#[derive(Clone, PartialEq, Debug)]
enum RowKind {
    Node,
    Loading,
    Error(String),
}

/// 扁平化之后的一行。`key` 是树的稳定身份，状态行用 `id::state` 区分。
#[derive(Clone, PartialEq, Debug)]
struct TreeRow {
    key: String,
    label: String,
    icon: Option<IconName>,
    badge: Option<String>,
    disabled: bool,
    branch: bool,
    expanded: bool,
    depth: usize,
    level: usize,
    parent: Option<String>,
    first_child: Option<String>,
    kind: RowKind,
}

fn is_branch(node: &TreeNode) -> bool {
    !node.children.is_empty() || node.state.is_some()
}

fn flatten(
    nodes: &[TreeNode],
    expanded: &HashSet<String>,
    parent: Option<&str>,
    depth: usize,
    out: &mut Vec<TreeRow>,
) {
    // 值树不可能成环，但深度护栏能挡住调用方手搓出来的畸形数据。
    if depth > 64 {
        return;
    }
    for node in nodes {
        let branch = is_branch(node);
        let open = branch && expanded.contains(&node.id);
        out.push(TreeRow {
            key: node.id.clone(),
            label: node.label.clone(),
            icon: node.icon,
            badge: node.badge.clone(),
            disabled: node.disabled,
            branch,
            expanded: open,
            depth,
            level: depth + 1,
            parent: parent.map(str::to_string),
            first_child: node.children.first().map(|c| c.id.clone()),
            kind: RowKind::Node,
        });
        if !open {
            continue;
        }
        match &node.state {
            Some(TreeState::Loading) => out.push(TreeRow {
                key: format!("{}::state", node.id),
                label: "正在加载…".to_string(),
                icon: None,
                badge: None,
                disabled: true,
                branch: false,
                expanded: false,
                depth: depth + 1,
                level: depth + 2,
                parent: Some(node.id.clone()),
                first_child: None,
                kind: RowKind::Loading,
            }),
            Some(TreeState::Error(message)) => out.push(TreeRow {
                key: format!("{}::state", node.id),
                label: message.clone(),
                icon: None,
                badge: None,
                disabled: true,
                branch: false,
                expanded: false,
                depth: depth + 1,
                level: depth + 2,
                parent: Some(node.id.clone()),
                first_child: None,
                kind: RowKind::Error(message.clone()),
            }),
            None => flatten(&node.children, expanded, Some(&node.id), depth + 1, out),
        }
    }
}

fn collect_branches(nodes: &[TreeNode], out: &mut HashSet<String>) {
    for node in nodes {
        if is_branch(node) {
            out.insert(node.id.clone());
        }
        collect_branches(&node.children, out);
    }
}

/// 下一帧再执行。Leptos 的展开 / 收起在微任务里落定，焦点搬运要在它之后。
fn schedule(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}

fn focus_row(root: &web_sys::Element, key: &str) -> Option<web_sys::HtmlElement> {
    let list = root.query_selector_all(".c-tree__row[data-key]").ok()?;
    for i in 0..list.length() {
        let Some(node) = list.get(i) else { continue };
        let Ok(el) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if el.get_attribute("data-key").as_deref() == Some(key) {
            let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() else {
                continue;
            };
            focus_without_scroll(&html);
            return Some(html);
        }
    }
    None
}

/// 聚焦，但不要让浏览器自己滚。
///
/// `focus()` 会顺手把目标滚进视口，而且 Chrome 是**居中**滚：连续按 ↓ 时，
/// 行每碰到下边缘就被拽到视口正中，看起来像一格一格地跳。滚动统一交给
/// `scroll_row_into_view`（只滚到刚好可见），焦点搬运就只搬焦点。
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

/// 焦点是否还在容器里。行被移出窗口后浏览器会把焦点丢回 body，这一步用来发现它。
fn holds_focus(root: &web_sys::Element) -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .is_some_and(|a| root.contains(Some(a.unchecked_ref::<web_sys::Node>())))
}

/// 焦点掉到 body 时收回容器：虚拟模式下焦点行随时可能被窗口摘掉，
/// 不收回来键盘事件就没人接了。只在「确实落到 body」时动手，
/// 免得抢走用户刚点到别处的焦点。
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

/// 把一行滚进视口。只滚组件自己的滚动容器：`scrollIntoView` 会连页面一起滚，
/// 而这里要的只是「窗口算得出来、行也在 DOM 里」。
fn scroll_row_into_view(container: &web_sys::Element, row: &web_sys::Element) {
    let c = container.get_bounding_client_rect();
    let r = row.get_bounding_client_rect();
    let offset = r.top() - c.top();
    let top = container.scroll_top() as f64;
    if offset < 0.0 {
        container.set_scroll_top((top + offset).max(0.0) as i32);
    } else if offset + r.height() > c.height() {
        container.set_scroll_top((top + offset + r.height() - c.height()).ceil() as i32);
    }
}

#[component]
pub fn TreeView(
    #[prop(into)] nodes: Vec<TreeNode>,
    /// 全部展开的初值；`default_expanded` 会在它之上再补几条。
    #[prop(optional)] default_open: bool,
    #[prop(into, optional)] default_expanded: Vec<String>,
    #[prop(into, optional)] selected: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] on_activate: Option<Callback<String>>,
    /// 展开 / 收起回调。调用方拿它做懒加载或持久化。
    #[prop(optional)] on_toggle: Option<Callback<(String, bool)>>,
    #[prop(optional)] on_retry: Option<Callback<String>>,
    #[prop(into, optional)] label: String,
    /// 开启虚拟行时必须给固定行高；CSS 的 `--tree-row-h` 与窗口计算共用它。
    #[prop(optional)] virtual_row_height: Option<f64>,
    /// >0 时组件自己成为滚动视口（`overflow-y:auto; max-height:Npx`）。
    /// 0 表示不限制：高度由外部容器给，行为与以前一致。
    #[prop(default = 0.0)] max_height: f64,
) -> impl IntoView {
    let label = if label.is_empty() {
        "树".to_string()
    } else {
        label
    };

    // 两个条件都满足才真正虚拟化：只给行高没有视口，量不出「该渲染哪几行」。
    let row_h = virtual_row_height.unwrap_or(26.0).max(1.0);
    let virtual_mode = virtual_row_height.is_some() && max_height > 0.0;

    let root_ref = NodeRef::<html::Div>::new();

    let init_expanded = {
        let mut set = HashSet::new();
        if default_open {
            collect_branches(&nodes, &mut set);
        }
        for id in default_expanded {
            set.insert(id);
        }
        set
    };
    let expanded = RwSignal::new(init_expanded.clone());

    let rows_nodes = nodes.clone();
    let rows = Memo::new(move |_| {
        let mut out = Vec::new();
        flatten(&rows_nodes, &expanded.get(), None, 0, &mut out);
        out
    });

    let nav_nodes = nodes.clone();
    let nav_ids = Memo::new(move |_| {
        let mut out = Vec::new();
        flatten(&nav_nodes, &expanded.get(), None, 0, &mut out);
        out.into_iter()
            .filter(|r| matches!(r.kind, RowKind::Node) && !r.disabled)
            .map(|r| r.key)
            .collect::<Vec<String>>()
    });

    let (initial_focus, initial_total) = {
        let mut out = Vec::new();
        flatten(&nodes, &init_expanded, None, 0, &mut out);
        let total = out.len();
        let focus = out
            .into_iter()
            .find(|r| matches!(r.kind, RowKind::Node) && !r.disabled)
            .map(|r| r.key);
        (focus, total)
    };
    let focused = RwSignal::new(initial_focus);

    // 收起父节点后焦点可能落在一个已经不可见的子行上；此时 roving tabindex
    // 必须退到第一个可导航行，否则整棵树会被 Tab 跳过。
    let effective_focus = Memo::new(move |_| {
        let nav = nav_ids.get();
        match focused.get() {
            Some(key) if nav.iter().any(|k| k == &key) => Some(key),
            _ => nav.first().cloned(),
        }
    });

    /* ------------------------------------------------------------ 虚拟窗口 */

    // 窗口算法在 `base/virtual_window`：这里只负责「按行号切片」与「撑高」。
    let total_rows = Memo::new(move |_| rows.get().len());
    let win = RwSignal::new(if virtual_mode {
        // 首帧还没有滚动事件，先按 max_height 估一个够用的窗口：
        // 行高是固定的，估出来只会偏大（被 total 夹住），不会露白。
        let visible = (max_height / row_h).ceil() as usize + 2 * virtual_window::OVERSCAN + 1;
        Window::new(0, visible.min(initial_total))
    } else {
        Window::full(initial_total)
    });
    let clamp_win = move || virtual_window::clamp(win.get(), total_rows.get());

    let visible_rows = Memo::new(move |_| {
        let all = rows.get();
        if !virtual_mode {
            return all;
        }
        let w = virtual_window::clamp(win.get(), all.len());
        all[w.slice()].to_vec()
    });

    // 折叠 / 换 items 之后行数会变：窗口必须夹回有效范围，否则切片越界。
    Effect::new(move |_| {
        if !virtual_mode {
            return;
        }
        let total = total_rows.get();
        let next = virtual_window::clamp(win.get_untracked(), total);
        if win.get_untracked() != next {
            win.set(next);
        }
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
                let next = virtual_window::window_of(
                    &el,
                    row_h,
                    total_rows.get_untracked(),
                    virtual_window::OVERSCAN,
                );
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
        let y = row as f64 * row_h;
        let next = if y < top {
            y
        } else if y + row_h > top + vh {
            y + row_h - vh
        } else {
            top
        };
        if next != top {
            root.set_scroll_top(next.max(0.0).round() as i32);
        }
        let next = virtual_window::window_of(
            root,
            row_h,
            total_rows.get_untracked(),
            virtual_window::OVERSCAN,
        );
        if win.get_untracked() != next {
            win.set(next);
        }
    };

    // 焦点搬运的唯一入口。虚拟模式下目标行可能不在 DOM 里：先滚进窗口，
    // 再聚焦；DOM 还没更新完就把焦点临时挂在容器上，下一帧补一次。
    let focus_key = move |key: String| {
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        if !virtual_mode {
            // 非虚拟路径保持旧节奏：行还没落定就下一帧再搬一次。
            if focus_row(&root, &key).is_none() {
                let root_later = root.clone();
                schedule(move || {
                    focus_row(&root_later, &key);
                });
            }
            return;
        }
        let idx = rows.get_untracked().iter().position(|r| r.key == key);
        if let Some(idx) = idx {
            if !clamp_win().contains(idx) {
                ensure_row(&root, idx);
            }
        }
        match focus_row(&root, &key) {
            Some(el) => scroll_row_into_view(&root, &el),
            None => {
                if let Ok(html) = root.clone().dyn_into::<web_sys::HtmlElement>() {
                    let _ = html.focus();
                }
                let root_later = root.clone();
                let key_later = key.clone();
                schedule(move || {
                    // 焦点已经被搬到别处时，这次补焦作废。
                    if effective_focus.get_untracked().as_deref() != Some(key_later.as_str()) {
                        return;
                    }
                    if let Some(el) = focus_row(&root_later, &key_later) {
                        scroll_row_into_view(&root_later, &el);
                    }
                });
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
        // 行被移出 DOM 时焦点会掉到 body：先记下「刚才焦点还在树里」。
        let had_focus = holds_focus(&root);
        let next = virtual_window::window_of(
            &root,
            row_h,
            total_rows.get_untracked(),
            virtual_window::OVERSCAN,
        );
        if win.get_untracked() != next {
            win.set(next);
        }
        // 焦点行被滚出窗口后，键盘事件就没人接了：把焦点交给容器，
        // 下一次按键再按 `focused` 里的行号把焦点搬回去。
        // DOM 更新是异步的（行摘掉之后焦点才掉到 body），所以要等下一帧再收。
        if had_focus {
            let root_later = root.clone();
            schedule(move || reclaim_focus(&root_later));
        }
    };

    let toggle = move |id: &str| {
        let open_now = !expanded.get_untracked().contains(id);
        expanded.update(|set| {
            if open_now {
                set.insert(id.to_string());
            } else {
                set.remove(id);
            }
        });
        if let Some(cb) = on_toggle {
            cb.run((id.to_string(), open_now));
        }
        // 收起可能把当前焦点行一起移出可见序列（鼠标点 twisty 时焦点还停在子行）。
        // 这时把焦点搬到第一个可导航行，别让它掉到 body 上。
        let nav = nav_ids.get_untracked();
        let stale = focused
            .get_untracked()
            .map_or(true, |k| !nav.iter().any(|x| x == &k));
        if stale {
            // 优先落回被操作的父行；它被禁用或不在序列里才退到第一行。
            let fallback = if nav.iter().any(|x| x == id) {
                Some(id.to_string())
            } else {
                nav.first().cloned()
            };
            if let Some(next) = fallback {
                focused.set(Some(next.clone()));
                focus_key(next);
            }
        }
    };

    let row_view = {
        let selected = selected;
        let on_select = on_select;
        let on_activate = on_activate;
        let on_retry = on_retry;
        let toggle = toggle;
        move |row: TreeRow| -> AnyView {
            let row_for_state = row.clone();
            let is_state = !matches!(row.kind, RowKind::Node);
            let key_tab = row.key.clone();
            let key_click = row.key.clone();
            let key_select = row.key.clone();
            let key_activate = row.key.clone();
            let key_twist = row.key.clone();
            // 状态行的 key 是「父节点::state」；重试要回到父节点，而不是这个占位行。
            let key_retry = row.parent.clone().unwrap_or_else(|| row.key.clone());
            let depth = row.depth;
            let level = row.level;
            let label_text = row.label.clone();
            let icon = row.icon;
            let badge = row.badge.clone();
            let disabled = row.disabled;
            let branch = row.branch;
            let expanded_now = row.expanded;
            let loading = matches!(row.kind, RowKind::Loading);
            let error = matches!(row.kind, RowKind::Error(_));

            if is_state {
                return view! {
                    <div
                        class="c-tree__row c-tree__row--state"
                        role="treeitem"
                        data-key=row_for_state.key.clone()
                        {..custom_attribute("aria-level", level.to_string())}
                        aria-disabled="true"
                        aria-busy=loading.then_some("true")
                        data-state=if loading { "loading" } else { "error" }
                        tabindex="-1"
                        style=format!("--tree-depth:{depth}")
                    >
                        <span class="c-tree__indent" aria-hidden="true"></span>
                        <span class="c-tree__twisty" aria-hidden="true"></span>
                        <span class="c-tree__state">{label_text}</span>
                        {error.then(|| {
                            view! {
                                <button
                                    class="c-tree__retry"
                                    type="button"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        if let Some(cb) = on_retry {
                                            cb.run(key_retry.clone());
                                        }
                                    }
                                >
                                    "重试"
                                </button>
                            }
                        })}
                    </div>
                }
                .into_any();
            }

            let selected = selected;
            view! {
                <div
                    class="c-tree__row"
                    role="treeitem"
                    data-key=row_for_state.key.clone()
                    {..custom_attribute("aria-level", level.to_string())}
                    aria-selected=move || (selected.get().as_deref() == Some(key_select.as_str())).to_string()
                    aria-expanded=branch.then(|| expanded_now.to_string())
                    aria-disabled=disabled.then_some("true")
                    aria-busy=loading.then_some("true")
                    tabindex=move || {
                        if effective_focus.get().as_deref() == Some(key_tab.as_str()) {
                            "0"
                        } else {
                            "-1"
                        }
                    }
                    style=format!("--tree-depth:{depth}")
                    on:click=move |ev: web_sys::MouseEvent| {
                        if disabled {
                            return;
                        }
                        if let Some(target) = ev.current_target() {
                            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                let _ = el.focus();
                            }
                        }
                        focused.set(Some(key_click.clone()));
                        if let Some(cb) = on_select {
                            cb.run(key_click.clone());
                        }
                    }
                    on:dblclick=move |_| {
                        if disabled {
                            return;
                        }
                        if let Some(cb) = on_activate {
                            cb.run(key_activate.clone());
                        }
                    }
                >
                    <span class="c-tree__indent" aria-hidden="true"></span>
                    <span
                        class="c-tree__twisty"
                        aria-hidden="true"
                        on:click=move |ev: web_sys::MouseEvent| {
                            if !branch || disabled {
                                return;
                            }
                            ev.stop_propagation();
                            toggle(&key_twist);
                        }
                    >
                        {branch.then(|| view! { <Icon name=IconName::CHEVRON_RIGHT size=12 /> })}
                    </span>
                    {icon.map(|name| {
                        view! {
                            <span class="c-tree__icon" aria-hidden="true">
                                <Icon name=name size=14 />
                            </span>
                        }
                    })}
                    <span class="c-tree__label">{label_text}</span>
                    {badge.filter(|b| !b.is_empty()).map(|b| {
                        view! { <span class="c-tree__badge">{b}</span> }
                    })}
                </div>
            }
            .into_any()
        }
    };

    let keydown = move |ev: web_sys::KeyboardEvent| {
        // 行内的真按钮（重试）自己处理按键，方向键不该抢。
        if let Some(target) = ev.target() {
            if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                if el
                    .closest("button, a, input, textarea, select")
                    .ok()
                    .flatten()
                    .is_some()
                {
                    return;
                }
            }
        }
        let Some(current) = effective_focus.get_untracked() else {
            return;
        };
        let nav = nav_ids.get_untracked();
        let Some(pos) = nav.iter().position(|k| k == &current) else {
            return;
        };
        let all = rows.get_untracked();
        let Some(row) = all
            .iter()
            .find(|r| r.key == current && matches!(r.kind, RowKind::Node))
        else {
            return;
        };

        // 用户可能先用滚轮把焦点行滚出了渲染窗口（此时焦点挂在容器上）。
        // 按键要按「焦点行」算，就得先把它拉回视口，否则下一次方向键会从看不见的位置继续走。
        if virtual_mode {
            if let Some(idx) = all.iter().position(|r| r.key == current) {
                if !clamp_win().contains(idx) {
                    if let Some(root) = root_ref.get_untracked() {
                        ensure_row(&root, idx);
                        focus_row(&root, &current);
                    }
                }
            }
        }

        let mut focus_to: Option<String> = None;
        let handled = match ev.key().as_str() {
            "ArrowDown" => {
                focus_to = nav.get(pos + 1).cloned();
                true
            }
            "ArrowUp" => {
                focus_to = nav.get(pos.saturating_sub(1)).cloned().filter(|_| pos > 0);
                true
            }
            "Home" => {
                focus_to = nav.first().cloned();
                true
            }
            "End" => {
                focus_to = nav.last().cloned();
                true
            }
            "ArrowRight" => {
                if row.branch && !row.expanded {
                    toggle(&current);
                } else if let Some(first) = row.first_child.clone() {
                    // 第一个子节点可能被禁用（不在导航序列里），此时原地不动。
                    if nav.iter().any(|k| k == &first) {
                        focus_to = Some(first);
                    }
                }
                true
            }
            "ArrowLeft" => {
                if row.branch && row.expanded {
                    toggle(&current);
                } else if let Some(parent) = row.parent.clone() {
                    if nav.iter().any(|k| k == &parent) {
                        focus_to = Some(parent);
                    }
                }
                true
            }
            " " => {
                if !row.disabled {
                    if let Some(cb) = on_select {
                        cb.run(current.clone());
                    }
                }
                true
            }
            "Enter" => {
                if !row.disabled {
                    if let Some(cb) = on_activate {
                        cb.run(current.clone());
                    }
                }
                true
            }
            _ => false,
        };
        if !handled {
            return;
        }
        ev.prevent_default();
        if let Some(key) = focus_to {
            focused.set(Some(key.clone()));
            focus_key(key);
        }
    };

    view! {
        <div
            class="c-tree"
            role="tree"
            aria-label=label
            data-virtual=virtual_mode.then_some("1")
            style=virtual_mode.then(|| {
                format!("--tree-row-h:{row_h}px;--tree-max-h:{max_height}px")
            })
            tabindex=virtual_mode.then_some("-1")
            node_ref=root_ref
            on:keydown=keydown
            on:scroll=on_scroll
        >
            <Show when=move || rows.get().is_empty()>
                <div class="c-tree__empty">"暂无内容"</div>
            </Show>
            {virtual_mode.then(|| {
                view! {
                    <div
                        class="c-tree__spacer"
                        role="presentation"
                        aria-hidden="true"
                        style=move || format!("height:{}px", clamp_win().offset(row_h))
                    ></div>
                }
            })}
            <For
                each=move || visible_rows.get()
                key=|row| row.key.clone()
                children=move |row: TreeRow| row_view(row)
            />
            {virtual_mode.then(|| {
                view! {
                    <div
                        class="c-tree__spacer"
                        role="presentation"
                        aria-hidden="true"
                        style=move || format!(
                            "height:{}px",
                            clamp_win().tail(total_rows.get(), row_h),
                        )
                    ></div>
                }
            })}
        </div>
    }
}
