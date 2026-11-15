//! 文档页签 DocumentTabs —— 一排「文档会话」，不是「换一种看法」。
//!
//! 和 `Tabs` 的分工：Tabs 是同一个窗口里的视图切换（分段语义，没有未保存、
//! 没有固定页签、没有关闭确认）；DocumentTabs 每一颗是一个文档会话，带
//! dirty / pinned / closable，能关、能新建、能拖着换位置。两者的视觉共用
//! 「一条轨道 + 抬起来的那一颗」，但 DOM 契约各写各的（见 61-document-tabs.css）。
//!
//! 重排**不用 HTML5 拖放**：dragstart 会吞掉 click，页面里的选中逻辑就得
//! 再补一套「拖动结束要不要选中」的判定。这里用 pointer 事件 + 5px 阈值，
//! 没过阈值就是一次普通点击，过了才进拖拽。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};

/// 拖拽判定阈值（px）。小于它当点击 —— 触控板和鼠标都会有几像素的抖动，
/// 阈值太小会把「想点一下」读成「想拖」。
const DRAG_THRESHOLD: f64 = 5.0;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DocumentTab {
    pub id: String,
    pub label: String,
    pub dirty: bool,
    pub pinned: bool,
    pub closable: bool,
}

impl DocumentTab {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            dirty: false,
            pinned: false,
            closable: true,
        }
    }

    /// 有未保存改动（标题旁一颗小圆点）。
    pub fn dirty(mut self) -> Self {
        self.dirty = true;
        self
    }

    /// 固定页签：只剩图标、不参与拖动、排在整排最前 —— 这是重排的硬边界。
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    /// 不可关闭（比如会话入口页）。`closable=false` 与 `pinned` 都能挡住关闭，
    /// 区别是 pinned 连拖动都不参与。
    pub fn closable(mut self, on: bool) -> Self {
        self.closable = on;
        self
    }
}

#[derive(Clone, PartialEq, Debug)]
struct DragState {
    id: String,
    x0: f64,
    y0: f64,
    started: bool,
}

/// 文档页签。
///
/// 列表是**首屏快照**：关闭/重排都在组件内部完成，然后通过回调把结果告诉调用方。
/// 这样做是因为关闭与拖拽都会重建 DOM，如果调用方每次回调再回灌一份新 `Vec`，
/// 组件会被整体重建 —— 焦点、滚动位置、指针捕获全丢。
#[component]
pub fn DocumentTabs(
    #[prop(into)] tabs: Vec<DocumentTab>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] on_close: Option<Callback<String>>,
    #[prop(optional)] on_new: Option<Callback<()>>,
    #[prop(optional)] on_reorder: Option<Callback<Vec<String>>>,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let docs = RwSignal::new(normalize(tabs));
    let initial = docs.get_untracked();
    let current = RwSignal::new(
        value
            .get_untracked()
            .filter(|v| initial.iter().any(|d| &d.id == v))
            .or_else(|| initial.first().map(|d| d.id.clone())),
    );

    let root_ref = NodeRef::<html::Div>::new();
    let scroll_ref = NodeRef::<html::Div>::new();

    // 拖拽态：drag 存起点与是否已越过阈值；drop_on/drop_x 是指示线的开关与位置。
    let drag = RwSignal::new(None::<DragState>);
    let drop_on = RwSignal::new(false);
    let drop_x = RwSignal::new(0.0f64);
    let drag_id = RwSignal::new(None::<String>);
    // 拖动结束后浏览器还会补一次 click：吞掉它，否则「拖完顺手选中了别处」。
    let skip_click = RwSignal::new(false);

    let aria = if label.is_empty() {
        "文档".to_string()
    } else {
        label
    };
    let has_new = on_new.is_some();

    // 外部受控值优先：调用方改了 value（比如新建后自动切过去），内部跟上。
    Effect::new(move |_| {
        if let Some(v) = value.get() {
            if docs.get_untracked().iter().any(|d| d.id == v) {
                current.set(Some(v));
            }
        }
    });

    /* ---------------- 动作 ---------------- */

    let select = move |id: String| {
        if current.get_untracked().as_deref() == Some(id.as_str()) {
            return;
        }
        current.set(Some(id.clone()));
        // 选中项必须完整露出来：溢出时点一颗只露半截的页签，不该靠用户自己滚。
        if let (Some(root), Some(scroll)) = (root_ref.get_untracked(), scroll_ref.get_untracked()) {
            if let Some(tab) = tab_in(root.as_ref(), &id) {
                reveal_in(scroll.as_ref(), &tab);
            }
        }
        if let Some(cb) = on_select {
            cb.run(id);
        }
    };

    let close = move |id: String| {
        let list = docs.get_untracked();
        let Some(pos) = list.iter().position(|d| d.id == id) else {
            return;
        };
        let doc = list[pos].clone();
        if doc.pinned || !doc.closable {
            return;
        }
        // 关闭前先算好落点：右邻优先，没有右邻退左邻（和浏览器关标签一致）。
        let next = list
            .get(pos + 1)
            .map(|d| d.id.clone())
            .or_else(|| (pos > 0).then(|| list[pos - 1].id.clone()));
        let was_current = current.get_untracked().as_deref() == Some(id.as_str());
        let had_focus = root_ref
            .get_untracked()
            .map(|root| focus_inside(root.as_ref(), &id))
            .unwrap_or(false);
        // 整排页签的滚动位置：子节点被整批替换时浏览器可能把它归零，
        // 关完一颗后视图跳回最左边，读起来像「整排都换了」。
        let keep_scroll = scroll_ref
            .get_untracked()
            .map(|scroll| scroll.scroll_left())
            .unwrap_or(0);

        docs.update(|l| {
            l.remove(pos);
        });
        if was_current {
            current.set(next.clone());
            // 选中确实变了就通知调用方 —— 否则受控的 value 会指向一颗已经不存在的页签。
            if let Some(cb) = on_select {
                if let Some(n) = &next {
                    cb.run(n.clone());
                }
            }
        }
        if let Some(cb) = on_close {
            cb.run(id);
        }

        // 焦点不能掉到 body：键盘用户关完一颗还要继续按方向键。
        let root_ref = root_ref;
        let scroll_ref = scroll_ref;
        let next = next.clone();
        defer(move || {
            if let Some(scroll) = scroll_ref.get_untracked() {
                scroll.set_scroll_left(keep_scroll);
            }
            let Some(root) = root_ref.get_untracked() else {
                return;
            };
            let root_el: &web_sys::Element = root.as_ref();
            match next {
                Some(id) => {
                    if had_focus {
                        focus_tab(root_el, &id);
                    }
                    if let (Some(scroll), Some(tab)) =
                        (scroll_ref.get_untracked(), tab_in(root_el, &id))
                    {
                        reveal_in(scroll.as_ref(), &tab);
                    }
                }
                None if had_focus => {
                    // 一颗都不剩：焦点落到「新建」。
                    if let Ok(Some(btn)) = root_el.query_selector(".c-documenttabs__new") {
                        if let Ok(el) = btn.dyn_into::<web_sys::HtmlElement>() {
                            let _ = el.focus();
                        }
                    }
                }
                None => {}
            }
        });
    };

    // Alt+←/→：和相邻那颗换位。固定页签不参与，也不允许跨越它。
    let swap = move |id: String, step: isize| {
        let mut list = docs.get_untracked();
        let Some(i) = list.iter().position(|d| d.id == id) else {
            return;
        };
        let j = i as isize + step;
        if j < 0 || j >= list.len() as isize {
            return;
        }
        let j = j as usize;
        if list[i].pinned || list[j].pinned {
            return;
        }
        list.swap(i, j);
        let ids: Vec<String> = list.iter().map(|d| d.id.clone()).collect();
        docs.set(list);
        if let Some(cb) = on_reorder {
            cb.run(ids);
        }
        // 重建后焦点回到同一颗页签（不是回到原来那一格）。
        let root_ref = root_ref;
        defer(move || {
            if let Some(root) = root_ref.get_untracked() {
                focus_tab(root.as_ref(), &id);
            }
        });
    };

    /* ---------------- 渲染 ---------------- */

    let items = move || {
        let all = docs.get();
        if all.is_empty() {
            return view! {
                <div class="c-documenttabs__empty">"没有打开的文档"</div>
            }
            .into_any();
        }
        let total = all.len();
        all.into_iter()
            .map(|tab| {
                let id_click = tab.id.clone();
                let id_close = tab.id.clone();
                let id_sel = tab.id.clone();
                let id_tab = tab.id.clone();
                let id_cls = tab.id.clone();
                let id_drag = tab.id.clone();
                let id_kb = tab.id.clone();
                let title = if tab.dirty {
                    format!("{} — 未保存", tab.label)
                } else {
                    tab.label.clone()
                };
                let pinned = tab.pinned;
                let dirty = tab.dirty;
                let closable = tab.closable && !pinned;
                let label_close = tab.label.clone();
                // 固定页签只剩图标：标题必须由 aria-label 提供，否则读屏只听到「标签页」。
                let aria_label = pinned.then(|| tab.label.clone());

                view! {
                    <div
                        class=move || {
                            let mut c = String::from("c-documenttabs__tab");
                            if pinned {
                                c.push_str(" c-documenttabs__tab--pinned");
                            }
                            // 拖起时压暗缩小（61 分区的 .is-dragging），让出位置由指示线表达。
                            if drag_id.get().as_deref() == Some(id_cls.as_str()) {
                                c.push_str(" is-dragging");
                            }
                            c
                        }
                        role="tab"
                        data-docid=tab.id.clone()
                        title=title
                        aria-label=aria_label
                        aria-selected=move || {
                            (current.get().as_deref() == Some(id_sel.as_str())).to_string()
                        }
                        tabindex=move || {
                            if current.get().as_deref() == Some(id_tab.as_str()) { "0" } else { "-1" }
                        }
                        on:pointerdown={
                            let id = id_drag.clone();
                            move |ev: web_sys::PointerEvent| {
                                // 固定页签不参与拖动（它是重排的硬边界）
                                if ev.button() != 0 || pinned {
                                    return;
                                }
                                // 关闭钮自己处理按下，别在这里开拖
                                if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                    if target.closest(".c-documenttabs__close").ok().flatten().is_some() {
                                        return;
                                    }
                                }
                                // 指针捕获：move/up 会一直回到这颗页签，
                                // 不用在 window 上挂监听、也不会被别的元素截走。
                                if let Some(target) = ev.current_target() {
                                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                                        let _ = el.set_pointer_capture(ev.pointer_id());
                                    }
                                }
                                drag.set(Some(DragState {
                                    id: id.clone(),
                                    x0: ev.client_x() as f64,
                                    y0: ev.client_y() as f64,
                                    started: false,
                                }));
                            }
                        }
                        on:pointermove=move |ev: web_sys::PointerEvent| {
                            let Some(mut d) = drag.get_untracked() else {
                                return;
                            };
                            let dx = ev.client_x() as f64 - d.x0;
                            let dy = ev.client_y() as f64 - d.y0;
                            if !d.started {
                                if dx.abs() < DRAG_THRESHOLD {
                                    return;
                                }
                                // 竖向手势让给滚动：页签条只做横向重排
                                if dx.abs() <= dy.abs() {
                                    return;
                                }
                                d.started = true;
                                drag_id.set(Some(d.id.clone()));
                                drag.set(Some(d.clone()));
                            }
                            if ev.cancelable() {
                                ev.prevent_default();
                            }
                            if let Some(root) = root_ref.get_untracked() {
                                let (_, x) = drop_target(root.as_ref(), &docs.get_untracked(), &d.id, ev.client_x() as f64);
                                drop_x.set(x);
                                drop_on.set(true);
                            }
                        }
                        on:pointerup=move |ev: web_sys::PointerEvent| {
                            let Some(d) = drag.get_untracked() else {
                                return;
                            };
                            drag.set(None);
                            drop_on.set(false);
                            drag_id.set(None);
                            if !d.started {
                                // 没过阈值 = 一次点击，交给 click 处理
                                return;
                            }
                            // 吞掉紧随其后的 click
                            skip_click.set(true);
                            defer(move || skip_click.set(false));

                            let Some(root) = root_ref.get_untracked() else {
                                return;
                            };
                            let list = docs.get_untracked();
                            let (at, _) = drop_target(root.as_ref(), &list, &d.id, ev.client_x() as f64);
                            let Some(moved) = list.iter().find(|t| t.id == d.id).cloned() else {
                                return;
                            };
                            let mut order: Vec<DocumentTab> =
                                list.iter().filter(|t| t.id != d.id).cloned().collect();
                            order.insert(at.min(order.len()), moved);
                            let ids: Vec<String> = order.iter().map(|t| t.id.clone()).collect();
                            let before: Vec<String> = list.iter().map(|t| t.id.clone()).collect();
                            if ids == before {
                                return;
                            }
                            let keep_scroll = scroll_ref
                                .get_untracked()
                                .map(|scroll| scroll.scroll_left())
                                .unwrap_or(0);
                            docs.set(order);
                            if let Some(cb) = on_reorder {
                                cb.run(ids);
                            }
                            // 重排后焦点回到被拖的那一颗（不是落到 body），
                            // 并把滚动位置还原 —— 重排不该顺手换一个视野。
                            let id = d.id.clone();
                            let root_ref = root_ref;
                            let scroll_ref = scroll_ref;
                            defer(move || {
                                if let Some(scroll) = scroll_ref.get_untracked() {
                                    scroll.set_scroll_left(keep_scroll);
                                }
                                let Some(root) = root_ref.get_untracked() else {
                                    return;
                                };
                                let root_el: &web_sys::Element = root.as_ref();
                                focus_tab(root_el, &id);
                                if let (Some(scroll), Some(tab)) =
                                    (scroll_ref.get_untracked(), tab_in(root_el, &id))
                                {
                                    reveal_in(scroll.as_ref(), &tab);
                                }
                            });
                        }
                        on:pointercancel=move |_| {
                            drag.set(None);
                            drop_on.set(false);
                            drag_id.set(None);
                        }
                        on:click=move |_| {
                            if skip_click.get_untracked() {
                                skip_click.set(false);
                                return;
                            }
                            select(id_click.clone());
                        }
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            let key = ev.key();
                            // Ctrl/Cmd+W 关「当前」页签：W 的语义是窗口/文档，不是焦点那一颗
                            if (ev.ctrl_key() || ev.meta_key()) && key.eq_ignore_ascii_case("w") {
                                ev.prevent_default();
                                if let Some(cur) = current.get_untracked() {
                                    close(cur);
                                }
                                return;
                            }
                            if ev.alt_key() && (key == "ArrowLeft" || key == "ArrowRight") {
                                ev.prevent_default();
                                swap(
                                    id_kb.clone(),
                                    if key == "ArrowRight" { 1 } else { -1 },
                                );
                                return;
                            }
                            let list = docs.get_untracked();
                            if list.is_empty() {
                                return;
                            }
                            let at = list.iter().position(|t| t.id == id_kb).unwrap_or(0);
                            match key.as_str() {
                                "ArrowLeft" | "ArrowRight" => {
                                    ev.prevent_default();
                                    let step = if key == "ArrowRight" { 1isize } else { -1 };
                                    let next =
                                        (at as isize + step).rem_euclid(total as isize) as usize;
                                    let next_id = list[next].id.clone();
                                    select(next_id.clone());
                                    focus_now(root_ref, &next_id);
                                }
                                "Home" | "End" => {
                                    ev.prevent_default();
                                    let next = if key == "Home" { 0 } else { total - 1 };
                                    let next_id = list[next].id.clone();
                                    select(next_id.clone());
                                    focus_now(root_ref, &next_id);
                                }
                                "Enter" | " " => {
                                    ev.prevent_default();
                                    select(id_kb.clone());
                                }
                                // 旧库契约：Delete/Backspace 也关（文档页签的肌肉记忆）
                                "Delete" | "Backspace" => {
                                    ev.prevent_default();
                                    close(id_kb.clone());
                                }
                                _ => {}
                            }
                        }
                    >
                        {pinned.then(|| view! { <Icon name=IconName::FILE_TEXT size=13 /> })}
                        {(!pinned).then(|| view! {
                            <span class="c-documenttabs__label">
                                <span class="c-documenttabs__title">{tab.label.clone()}</span>
                                {dirty.then(|| view! {
                                    <span class="c-documenttabs__dirty" aria-hidden="true"></span>
                                })}
                            </span>
                        })}
                        {closable.then(|| view! {
                            <button
                                class="c-documenttabs__close"
                                type="button"
                                // 鼠标可达但不占 Tab 序列：键盘用 Ctrl+W / Delete 关
                                tabindex="-1"
                                aria-label=format!("关闭 {label_close}")
                                title=format!("关闭 {label_close}")
                                on:click={
                                    let id = id_close.clone();
                                    move |ev| {
                                        ev.stop_propagation();
                                        close(id.clone());
                                    }
                                }
                            >
                                <Icon name=IconName::X size=9 />
                            </button>
                        })}
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div
            class="c-documenttabs"
            role="tablist"
            aria-label=aria
            data-dragging=move || drag_id.get().is_some().then_some("1")
            node_ref=root_ref
        >
            <div class="c-documenttabs__scroll" node_ref=scroll_ref>
                {items}
            </div>
            {has_new.then(|| view! {
                <button
                    class="c-documenttabs__new"
                    type="button"
                    aria-label="新建文档"
                    title="新建文档"
                    on:click=move |_| {
                        if let Some(cb) = on_new {
                            cb.run(());
                        }
                    }
                >
                    <Icon name=IconName::PLUS size=12 />
                </button>
            })}
            <div
                class=move || {
                    if drop_on.get() {
                        "c-documenttabs__drop is-on"
                    } else {
                        "c-documenttabs__drop"
                    }
                }
                aria-hidden="true"
                style=move || format!("left:{}px", drop_x.get())
            ></div>
        </div>
    }
}

/* ---------------- 纯函数与小工具 ---------------- */

/// 去重 + 固定页签排前面。同 id 的第二颗页签没有意义，还会让选中指向两颗。
fn normalize(list: Vec<DocumentTab>) -> Vec<DocumentTab> {
    let mut out: Vec<DocumentTab> = Vec::new();
    for tab in list {
        if tab.id.is_empty() || out.iter().any(|t| t.id == tab.id) {
            continue;
        }
        out.push(tab);
    }
    out.sort_by_key(|t| !t.pinned);
    out
}

fn tabs_in(root: &web_sys::Element) -> Vec<web_sys::Element> {
    let mut out = Vec::new();
    let Ok(list) = root.query_selector_all(".c-documenttabs__tab") else {
        return out;
    };
    for i in 0..list.length() {
        if let Some(node) = list.item(i) {
            if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                out.push(el);
            }
        }
    }
    out
}

fn tab_in(root: &web_sys::Element, id: &str) -> Option<web_sys::Element> {
    tabs_in(root)
        .into_iter()
        .find(|el| el.get_attribute("data-docid").as_deref() == Some(id))
}

fn focus_tab(root: &web_sys::Element, id: &str) -> bool {
    let Some(el) = tab_in(root, id) else {
        return false;
    };
    match el.dyn_into::<web_sys::HtmlElement>() {
        Ok(el) => el.focus().is_ok(),
        Err(_) => false,
    }
}

fn focus_inside(root: &web_sys::Element, id: &str) -> bool {
    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element());
    let (Some(active), Some(tab)) = (active, tab_in(root, id)) else {
        return false;
    };
    tab.contains(Some(active.unchecked_ref::<web_sys::Node>()))
}

/// 把目标页签滚进可视区。只算 `__scroll` 自己的 scrollLeft，
/// 不用 `scrollIntoView` —— 那会把整页也一起滚走。
fn reveal_in(scroller: &web_sys::Element, tab: &web_sys::Element) {
    let s = scroller.get_bounding_client_rect();
    let t = tab.get_bounding_client_rect();
    let left = scroller.scroll_left() as f64;
    let view = scroller.client_width() as f64;
    // 页签在「内容坐标」里的位置：视口坐标差 + 当前滚动量
    let x = t.left() - s.left() + left;
    let right = x + t.width();
    if x < left {
        scroller.set_scroll_left((x - 8.0).max(0.0) as i32);
    } else if right > left + view {
        scroller.set_scroll_left((right - view + 8.0) as i32);
    }
}

/// 落点判定：**和其余页签的中点比**，第一个中点在被拖指针右侧的位置就是插入点。
///
/// 为什么不按「页签自身矩形」判：被拖那颗自己也在 DOM 里且位置不变，
/// 按矩形判会在它的左右边界各产生一个死区（拖到自家身上时算不出落点）。
/// 排除自己之后，每个位置都唯一。
///
/// 返回 (插入下标, 指示线在容器内的 x)。
fn drop_target(
    root: &web_sys::Element,
    list: &[DocumentTab],
    drag_id: &str,
    client_x: f64,
) -> (usize, f64) {
    let root_rect = root.get_bounding_client_rect();
    let others: Vec<&DocumentTab> = list.iter().filter(|t| t.id != drag_id).collect();
    let mut at = others.len();
    for (i, tab) in others.iter().enumerate() {
        if let Some(el) = tab_in(root, &tab.id) {
            let r = el.get_bounding_client_rect();
            if client_x < r.left() + r.width() / 2.0 {
                at = i;
                break;
            }
        }
    }
    // 固定页签是硬边界：插入点不可能落在它们前面
    let pinned = others.iter().filter(|t| t.pinned).count();
    if at < pinned {
        at = pinned;
    }
    let x = if at >= others.len() {
        others
            .last()
            .and_then(|tab| tab_in(root, &tab.id))
            .map(|el| el.get_bounding_client_rect().right() - root_rect.left())
            .unwrap_or(3.0)
    } else {
        tab_in(root, &others[at].id)
            .map(|el| el.get_bounding_client_rect().left() - root_rect.left())
            .unwrap_or(3.0)
    };
    (at, x)
}

/// 键盘切页后焦点跟着走。DOM 没重建（选中只是改属性），所以可以同步聚焦。
fn focus_now(root: NodeRef<html::Div>, id: &str) {
    if let Some(root) = root.get_untracked() {
        focus_tab(root.as_ref(), id);
    }
}

/// 下一帧做事。关闭/重排会重建 DOM，信号赋值那一刻新节点还没挂上 ——
/// 同一次同步调用里 focus() 会落空。
fn defer(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}
