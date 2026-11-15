//! 树：文件树 / 图层 / 大纲。
//!
//! 两条与「递归画 DOM」相反的取舍（沿用旧库结论）：
//! 1) 只渲染可见行：收起的子树根本不在 DOM 里，展开状态用 `HashSet` 管。
//! 2) 扁平 DOM + `aria-level`，不嵌套 `role="group"`：可见行本来就是一维序列，
//!    「下移一行」就是数组下标 +1，键盘逻辑不必进出子树。
//!
//! 展开态、焦点、选中分开存：收起父节点不该丢选中；方向键只移动焦点，
//! Space / 点击才改变选中。

use std::collections::HashSet;

use leptos::attr::custom::custom_attribute;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};

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

fn focus_row(root: &web_sys::Element, key: &str) {
    let Ok(list) = root.query_selector_all(".c-tree__row[data-key]") else {
        return;
    };
    for i in 0..list.length() {
        let Some(node) = list.get(i) else { continue };
        let Ok(el) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if el.get_attribute("data-key").as_deref() == Some(key) {
            if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = html.focus();
            }
            return;
        }
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
) -> impl IntoView {
    let label = if label.is_empty() {
        "树".to_string()
    } else {
        label
    };

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

    let initial_focus = {
        let mut out = Vec::new();
        flatten(&nodes, &init_expanded, None, 0, &mut out);
        out.into_iter()
            .find(|r| matches!(r.kind, RowKind::Node) && !r.disabled)
            .map(|r| r.key)
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
                schedule(move || {
                    if let Some(root) = root_ref.get_untracked() {
                        focus_row(&root, &next);
                    }
                });
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
            if let Some(root) = root_ref.get_untracked() {
                focus_row(&root, &key);
            }
        }
    };

    view! {
        <div
            class="c-tree"
            role="tree"
            aria-label=label
            node_ref=root_ref
            on:keydown=keydown
        >
            <Show when=move || rows.get().is_empty()>
                <div class="c-tree__empty">"暂无内容"</div>
            </Show>
            <For
                each=move || rows.get()
                key=|row| row.key.clone()
                children=move |row: TreeRow| row_view(row)
            />
        </div>
    }
}
