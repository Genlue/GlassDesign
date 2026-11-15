//! 检查器（右侧栏）：外壳 + 属性表 + 大纲。
//!
//! 四条取舍，都是「不测量、不重建」换来的：
//! 1. `Inspector` 不套玻璃档位。面板高度由内容决定，而贴图必须与元素边框盒
//!    逐像素一致（`feImage` 不会拉伸，见 04-交接 §4.3），套固定档位会硬切。
//!    要展示玻璃就在外面单独摆一块 `GlassPanel`，不要塞进检查器。
//! 2. `children` 只落在一个 `.c-inspector__pane` 里。Leptos 的 `Children` 是
//!    `FnOnce`，一份 children 拆不成多个 pane，所以「多视图」由调用方按
//!    `selected_tab` 自己换内容；组件只管外壳与切换。这也和本库其他控件
//!    同一条规矩：值受控，点击只上报。
//! 3. 页签复用 `Segmented`：`.c-inspector__tabs` 只负责把它拉满整行。
//! 4. 大纲是扁平 DOM + `aria-level`，缩进写 `--depth`。方向键的「下一行」
//!    就是数组下标 +1，不用进出子树，也不用假设 DOM 顺序。

use leptos::attr::custom::custom_attribute;
use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::widgets::progress::Empty;
use crate::widgets::segmented::{Segmented, SegmentedOption};

/// 检查器外壳。高度跟随宿主（`.c-inspector` 是 `height:100%`），
/// 所以调用方要给一个确定高度的盒子，例如 `style="width:320px;height:420px"`。
#[component]
pub fn Inspector(
    #[prop(into, optional)] title: String,
    /// 页签文案；页签值就是文案本身，`on_tab` 收到的是同一个字符串。
    #[prop(into)] tabs: Vec<String>,
    #[prop(into, optional)] selected_tab: MaybeProp<String>,
    #[prop(optional)] on_tab: Option<Callback<String>>,
    /// 页签右侧的动作区（收起按钮之类），由调用方决定放什么。
    #[prop(optional)] actions: Option<Children>,
    #[prop(into, optional)] empty: MaybeProp<bool>,
    children: Children,
) -> impl IntoView {
    let label = if title.is_empty() {
        "检查器".to_string()
    } else {
        title
    };
    let first = tabs.first().cloned().unwrap_or_default();
    // 没给 selected_tab 时高亮第一页：分段控件是受控的，
    // 没有初值就会变成「哪一页都没选中」的空轨道。
    let fallback = first.clone();
    let active = move || selected_tab.get().unwrap_or_else(|| fallback.clone());
    // 同一个 Signal 既喂给分段控件（高亮哪一项），也写到 pane 的 data-pane 上。
    let value = Signal::derive(active);
    let options = tabs
        .into_iter()
        .map(|t| SegmentedOption::new(t.clone(), t))
        .collect::<Vec<_>>();
    // Segmented 的 `on_change` 是 `#[prop(optional)] Option<Callback<_>>`：
    // 这种写法在 builder 里收的是 `Callback` 本身（默认 None），所以这里给一个
    // 空回调占位，调用方没接回调时行为等价于「只切视觉、不上报」。
    let tab_click = on_tab.unwrap_or_else(|| Callback::new(|_: String| {}));
    let is_empty = move || empty.get().unwrap_or(false);

    view! {
        <aside class="c-inspector" aria-label=label>
            <div class="c-inspector__head">
                <div class="c-inspector__tabs">
                    <Segmented options=options value=value on_change=tab_click />
                </div>
                {actions.map(|a| a())}
            </div>
            <div class="c-inspector__body">
                // 常驻 DOM + data-active 切显示，不用 Show 包 children：
                // Children 是 FnOnce，放进条件分支会在切页时被消费掉。
                <div
                    class="c-inspector__pane"
                    data-pane=move || value.get()
                    data-active=move || if is_empty() { "0" } else { "1" }
                >
                    {children()}
                </div>
                <Show when=is_empty>
                    <div class="c-inspector__empty">
                        <Empty
                            title="未选中对象"
                            desc="在画布上选择一个图层，这里显示它的属性。"
                        />
                    </div>
                </Show>
            </div>
        </aside>
    }
}

/// 属性分组：一个标题 + 若干 `PropRow`。
#[component]
pub fn PropList(#[prop(into)] title: String, children: Children) -> impl IntoView {
    let has_title = !title.is_empty();
    view! {
        <div class="c-props">
            {has_title.then(|| view! { <div class="c-props__title">{title.clone()}</div> })}
            <div class="c-props__group">{children()}</div>
        </div>
    }
}

/// 一行属性：左标签、右值。值是 `Children`，可以放纯文本，也可以放控件。
#[component]
pub fn PropRow(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <div class="c-props__row">
            <span class="c-props__label">{label}</span>
            // div 而不是 span：值区经常放滑块这类块级控件，span 里塞 div 是非法嵌套。
            <div class="c-props__value">{children()}</div>
        </div>
    }
}

/// 大纲的一行。`depth` 只影响缩进与字号，不改变语义（扁平序列）。
#[derive(Clone, PartialEq, Debug)]
pub struct OutlineItem {
    pub id: String,
    pub label: String,
    pub depth: u32,
    pub icon: Option<IconName>,
}

impl OutlineItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>, depth: u32) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            depth,
            icon: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// 大纲：文档结构的一列，单选。
///
/// `role="tree"` + 扁平的 `treeitem`；roving tabindex 保证整列只有一个 Tab 停点，
/// 方向键移动焦点并同时改选中（单选列表的标准行为，Space 不是必需的）。
#[component]
pub fn Outline(
    #[prop(into)] items: Vec<OutlineItem>,
    #[prop(into, optional)] selected: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
) -> impl IntoView {
    let refs: Vec<NodeRef<html::Div>> = (0..items.len()).map(|_| NodeRef::new()).collect();
    let refs_for_keys = refs.clone();
    let order: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
    let order_for_focus = order.clone();
    let first = order.first().cloned();

    // 焦点与选中分开存：调用方可能把选中换成列表外的一行（例如刚删掉），
    // 那时焦点不能跟着消失，否则整列会被 Tab 跳过。
    let focused = RwSignal::new(selected.get_untracked().or_else(|| first.clone()));
    let focus_key = Memo::new(move |_| match focused.get() {
        Some(key) if order_for_focus.iter().any(|k| k == &key) => Some(key),
        _ => first.clone(),
    });

    let rows = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let id_for_state = item.id.clone();
            let id_for_active = item.id.clone();
            let id_for_tab = item.id.clone();
            let id_for_click = item.id.clone();
            let row_ref = refs[i];
            let level = item.depth + 1;
            let depth = item.depth;
            let label = item.label.clone();
            view! {
                <div
                    class="c-outline__item"
                    role="treeitem"
                    data-id=item.id.clone()
                    data-level=level.to_string()
                    {..custom_attribute("aria-level", level.to_string())}
                    aria-selected=move || {
                        (selected.get().as_deref() == Some(id_for_state.as_str())).to_string()
                    }
                    data-active=move || {
                        if selected.get().as_deref() == Some(id_for_active.as_str()) { "1" } else { "0" }
                    }
                    tabindex=move || {
                        if focus_key.get().as_deref() == Some(id_for_tab.as_str()) { "0" } else { "-1" }
                    }
                    style=format!("--depth:{depth}")
                    node_ref=row_ref
                    on:click=move |_| {
                        focused.set(Some(id_for_click.clone()));
                        // 鼠标点完也要把真实焦点搬过来：roving tabindex 已经把
                        // tabindex=0 挪到这一行了，不 focus 的话下一次 Tab 会从 body 起算。
                        if let Some(el) = row_ref.get() {
                            let _ = el.focus();
                        }
                        if let Some(cb) = on_select { cb.run(id_for_click.clone()); }
                    }
                >
                    // 图标位永远占宽：混排「有图标 / 无图标」时，文字若不共享同一条
                    // 起始线，层级缩进会看起来忽左忽右（12px/级 的差异还没它大）。
                    <span class="c-outline__lead">
                        {item.icon.map(|name| view! { <Icon name=name size=13 /> })}
                    </span>
                    <span class="c-outline__text">{label}</span>
                </div>
            }
        })
        .collect::<Vec<_>>();

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let Some(current) = focus_key.get_untracked() else { return };
        let Some(pos) = order.iter().position(|k| k == &current) else { return };

        if ev.key() == "Enter" {
            ev.prevent_default();
            if let Some(cb) = on_select {
                cb.run(current);
            }
            return;
        }

        let next = match ev.key().as_str() {
            "ArrowDown" => (pos + 1).min(order.len().saturating_sub(1)),
            "ArrowUp" => pos.saturating_sub(1),
            "Home" => 0,
            "End" => order.len().saturating_sub(1),
            _ => return,
        };
        ev.prevent_default();
        let key = order[next].clone();
        focused.set(Some(key.clone()));
        // 焦点靠真实 DOM 调用搬，不靠「重建后浏览器碰巧把焦点留下」。
        if let Some(el) = refs_for_keys[next].get() {
            let _ = el.focus();
        }
        if let Some(cb) = on_select {
            cb.run(key);
        }
    };

    view! {
        <div class="c-outline" role="tree" aria-label="大纲" on:keydown=on_keydown>
            {rows}
        </div>
    }
}
