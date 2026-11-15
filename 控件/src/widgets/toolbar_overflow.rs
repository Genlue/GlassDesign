//! 工具栏溢出 ToolbarOverflow —— 一行放不下的次要动作收进「更多」。
//!
//! 为什么需要它：工具栏的宽度由窗口决定、动作数量由功能决定，两者永远对不上。
//! 让按钮自己压 flex 会挤成一坨；把放不下的项收进菜单，视觉上仍是一排整齐的图标。
//!
//! 三个坑，都按原因处理：
//!
//! 1. **先量后放**。收进菜单的项是 `hidden`（display:none），宽度读出来是 0。
//!    所以每轮排布先给容器挂 `data-measuring`（CSS 只做 `visibility:hidden`），
//!    把全部项按自然宽度铺开，量完再决定谁走。
//! 2. **可用宽度要在「全部展开」那一刻读**。容器在 flex 行里是 shrink-to-fit，
//!    藏掉几项后自己会跟着变窄；这时候读到的宽度是自己的回声，越藏越窄，
//!    最后只剩一颗「更多」。所以量宽度和量容器放在同一次测量态里。
//! 3. **不假设等宽**。每个按钮都实测 `getBoundingClientRect().width()`：
//!    图标按钮眼下恰好同宽，但一旦混进分隔线或带文字的按钮，等宽假设就崩了。
//!
//! 溢出菜单**不另写弹层**：直接复用 `Menu`（`fixed_size = false`），
//! Esc / z-index / 点外关闭 / 焦点归还全归 `OverlayHost` 与 `Menu` 管。

use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::base::icon::{Icon, IconName};
use crate::base::overlay::{active_element, listen_window, schedule, GlobalListener};
use crate::widgets::menu::{Menu, MenuItem};

/// 「更多」按钮与菜单之间的间距。与 menu.rs 的 TRIGGER_GAP 一致：
/// 同一个窗口里所有「按钮 → 菜单」的距离必须是同一个数。
const MORE_GAP: f64 = 6.0;
/// 默认横向间距。53 分区的 CSS 兜底是 `--s-2`(4)，但图标按钮之间 6 更透气。
const DEFAULT_GAP: f64 = 6.0;

/// 工具栏上的一个动作。按钮只有图标，`label` 同时是 `aria-label` 与菜单文案。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ToolbarAction {
    pub id: String,
    pub label: String,
    pub icon: IconName,
    pub disabled: bool,
}

impl ToolbarAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            disabled: false,
        }
    }

    /// 不可用：按钮灰掉、菜单项同样灰掉（不是「藏起来」—— 位置要稳定）。
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// 一行会自动收纳的工具栏。
///
/// # 键盘
/// - `←` / `→` 在可见按钮之间移动（首尾循环），roving tabindex 保证整排只有一个 Tab 位；
/// - `Home` / `End` 到第一颗 / 「更多」（「更多」永远排在最后）；
/// - `Enter` / `Space` 激活当前按钮（原生 button 行为，不必自己写）。
///
/// # 降级
/// 量不到宽度（祖先 `display:none`、还没布局）时**整排显示**：宁可溢出，
/// 也不要让工具栏只剩一颗「更多」。
#[component]
pub fn ToolbarOverflow(
    #[prop(into)] actions: Vec<ToolbarAction>,
    /// 收纳顺序：列在这里的先被收进菜单，越靠前越先走；没列到的从尾部往前收。
    #[prop(into, optional)] priority: Vec<String>,
    /// 至少留几颗在工具栏上（默认 1）。兜底用：窗口窄到放不下一颗也不该只剩「更多」。
    #[prop(optional)] min_visible: usize,
    #[prop(optional)] gap: f64,
    #[prop(optional)] on_action: Option<Callback<String>>,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let gap = if gap > 0.0 { gap } else { DEFAULT_GAP };
    let min_visible = min_visible.max(1);
    let aria = if label.is_empty() {
        "工具栏".to_string()
    } else {
        label
    };

    let root_ref = NodeRef::<html::Div>::new();
    let more_ref = NodeRef::<html::Button>::new();

    // 已被收进菜单的 id（DOM 顺序）。空 = 全部放得下。
    let hidden = RwSignal::new(Vec::<String>::new());
    // roving tabindex 的持有者。收起来的那一颗不能继续占着 Tab 位。
    let active = RwSignal::new(actions.first().map(|a| a.id.clone()));
    let open = RwSignal::new(false);
    let anchor = RwSignal::new((0.0f64, 0.0f64, 0.0f64));
    // pointerdown 时菜单是否已经开着：Menu 的点外监听先关、紧接着的 click
    // 再 toggle 就会「关了又开」，看起来像点了个寂寞（MenuButton 里同一个坑）。
    let pressed_open = StoredValue::new_local(false);
    let resize = StoredValue::new_local(None::<GlobalListener>);
    // 外层容器（工具栏）的宽度。ResizeObserver 靠它去重：
    // 我们隐藏按钮会让「自己」变窄，父级宽度却不变 —— 用父级宽度判断
    // 「是不是外面真的变了」，避免一条永远停不下来的重排循环。
    let outer = StoredValue::new_local(0.0f64);
    let observer = StoredValue::new_local(None::<SizeObserver>);

    /* ---------------- 排布 ---------------- */

    let priority_for_layout = priority.clone();
    let relayout = Rc::new(move || {
        let (Some(root), Some(more)) = (root_ref.get_untracked(), more_ref.get_untracked()) else {
            return;
        };
        outer.set_value(outer_width(&root));
        let next = layout(&root, &more, &priority_for_layout, gap, min_visible);
        let changed = hidden.get_untracked() != next;
        // 菜单里的项是「隐藏集合」的快照，集合一变它就开始说谎；
        // 收起来（而不是重建）是最省事的正确做法。
        if changed && open.get_untracked() {
            open.set(false);
        }
        if changed {
            hidden.set(next);
        }

        // roving：Tab 位必须落在看得见、按得动的那一颗上，
        // 否则整条工具栏会从 Tab 序列里消失。
        let reachable = item_elements(&root)
            .iter()
            .filter(|el| !el.has_attribute("hidden") && !el.has_attribute("disabled"))
            .filter_map(attr_id)
            .collect::<Vec<String>>();
        let cur = active.get_untracked();
        if !cur.as_ref().is_some_and(|id| reachable.contains(id)) {
            active.set(reachable.into_iter().next());
        }
    });

    // 窗口尺寸变化重算：rAF 合并，拖窗口时一帧最多排一次。
    {
        let relayout = relayout.clone();
        if let Some(listener) = listen_window("resize", false, move |_| {
            let relayout = relayout.clone();
            schedule(move || relayout());
        }) {
            resize.set_value(Some(listener));
        }
    }

    // 挂载后量一次，并装上 ResizeObserver。
    // 只监听 window.resize 不够：工具栏宽度常常由 max-width / 侧栏开合决定，
    // 窗口没变而容器变了，不重排的话就会「明明放不下却全挤在外面」。
    {
        let relayout = relayout.clone();
        Effect::new(move |_| {
            let relayout = relayout.clone();
            schedule(move || {
                relayout();
                install_observer(&observer, root_ref, relayout.clone(), &outer);
            });
        });
    }

    on_cleanup(move || {
        // 显式摘监听：owner 释放时 StoredValue 也会 Drop，但这里写出来更清楚
        // 「window 监听必须跟着组件走」。ResizeObserver 随 observer Drop 断开。
        resize.set_value(None);
        observer.set_value(None);
    });

    /* ---------------- 交互 ---------------- */

    let open_menu = move || {
        if let Some(el) = more_ref.get_untracked() {
            // 先把焦点放到触发按钮：Menu 关闭时靠 activeElement 归还焦点，
            // 程序化打开（或某些指针设备）不一定已经把焦点放上去。
            let _ = el.focus();
            let r = el.get_bounding_client_rect();
            anchor.set((r.left(), r.bottom() + MORE_GAP, r.width()));
        }
        open.set(true);
    };

    // ←/→ 在可见按钮之间搬焦点；「更多」排在最后，所以 End 天然落在它身上。
    let on_key = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if !matches!(
            key.as_str(),
            "ArrowLeft" | "ArrowRight" | "Home" | "End"
        ) {
            return;
        }
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        let list = focusable_buttons(&root);
        if list.is_empty() {
            return;
        }
        let cur = active_element();
        let at = cur
            .as_ref()
            .map(|el| {
                let node: &web_sys::Node = el;
                list.iter().position(|b| b.is_same_node(Some(node)))
            })
            .unwrap_or(None);

        let len = list.len() as isize;
        let next = match key.as_str() {
            "Home" => 0,
            // 最后一颗就是「更多」：End = 直达溢出菜单。
            "End" => len - 1,
            "ArrowLeft" => at.map(|i| (i as isize - 1).rem_euclid(len)).unwrap_or(len - 1),
            _ => at.map(|i| (i as isize + 1).rem_euclid(len)).unwrap_or(0),
        };
        ev.prevent_default();
        let el = &list[next as usize];
        let _ = el.focus();
        // 「更多」不参与 roving（它常驻 tabindex=-1）：焦点路过不该把 Tab 位带走。
        if let Some(id) = attr_id(el) {
            active.set(Some(id));
        }
    };

    /* ---------------- 渲染 ---------------- */

    let buttons = actions
        .iter()
        .map(|action| {
            let focus_id = action.id.clone();
            let tab_id = action.id.clone();
            let click_id = action.id.clone();
            let label = action.label.clone();
            let icon = action.icon;
            let disabled = action.disabled;
            view! {
                <button
                    class="c-iconbtn"
                    type="button"
                    data-oa-id=action.id.clone()
                    // 图标按钮没有文字，可读名必须显式给；title 同时当工具提示。
                    aria-label=label.clone()
                    title=label
                    disabled=disabled
                    tabindex=move || {
                        if active.get().as_deref() == Some(tab_id.as_str()) { "0" } else { "-1" }
                    }
                    on:focus=move |_| active.set(Some(focus_id.clone()))
                    on:click=move |_| {
                        if disabled {
                            return;
                        }
                        if let Some(cb) = on_action {
                            cb.run(click_id.clone());
                        }
                    }
                >
                    <Icon name=icon size=16 />
                </button>
            }
        })
        .collect_view();

    let menu_actions = actions.clone();
    // 菜单项的派生值：隐藏集合一变就重算。
    // 用 `Memo`（Copy）而不是闭包，是因为 Portal 的 children 必须是 `Fn`，
    // 直接把 Vec 捕获进渲染闭包就只能调用一次。
    let menu_items_memo = Memo::new(move |_| {
        let hid = hidden.get();
        menu_items(&menu_actions, &hid)
    });

    view! {
        <div
            class="c-toolbaroverflow"
            role="toolbar"
            aria-orientation="horizontal"
            aria-label=aria
            style=format!("gap:{gap}px")
            node_ref=root_ref
            on:keydown=on_key
        >
            {buttons}
            <button
                class="c-iconbtn c-toolbaroverflow__more"
                type="button"
                aria-label="更多"
                title="更多"
                aria-haspopup="menu"
                aria-expanded=move || open.get().to_string()
                // 常驻 -1：它不属于 roving 序列，用 End 或点击到达。
                tabindex="-1"
                hidden=true
                node_ref=more_ref
                on:pointerdown=move |_| pressed_open.set_value(open.get_untracked())
                on:click=move |_| {
                    if pressed_open.get_value() {
                        // 这次 click 只是「点外关闭」的尾巴，不要再开一次。
                        pressed_open.set_value(false);
                        return;
                    }
                    if open.get_untracked() {
                        open.set(false);
                    } else {
                        open_menu();
                    }
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if matches!(ev.key().as_str(), "ArrowDown" | "ArrowUp") && !open.get_untracked() {
                        ev.prevent_default();
                        open_menu();
                    }
                }
            >
                <Icon name=IconName::MORE size=16 />
            </button>
        </div>

        // 菜单项跟着「隐藏集合」变，所以整块放进响应式闭包里重建。
        // 只在隐藏集合变化时重建，而那时菜单已经关掉，不存在「打开中被换掉」。
        //
        // 必须走 `Portal`：本组件几乎总是放在 `.c-toolbar` 里，而它的
        // `backdrop-filter` 会给 `position:fixed` 造一个包含块 —— 菜单会按工具栏
        // 的坐标摆、还会跟着页面滚，甚至为露出菜单项把整页滚走。
        // 挂到 body 之后，fixed 才真的是视口坐标。
        // 菜单项跟着「隐藏集合」变，所以整块放进响应式闭包里重建。
        // 只在隐藏集合变化时重建，而那时菜单已经关掉，不存在「打开中被换掉」。
        //
        // 必须走 `Portal`：本组件几乎总是放在 `.c-toolbar` 里，而它的
        // `backdrop-filter` 会给 `position:fixed` 造一个包含块 —— 菜单会按工具栏
        // 的坐标摆、还会跟着页面滚，甚至为露出菜单项把整页滚走。
        // 挂到 body 之后，fixed 才真的是视口坐标。
        <Portal>
            {move || {
                let items = menu_items_memo.get();
                view! {
                    <Menu
                        open=Signal::derive(move || open.get())
                        anchor=anchor
                        items=items
                        label="更多操作"
                        on_select=Callback::new(move |id: String| {
                            if let Some(cb) = on_action {
                                cb.run(id);
                            }
                        })
                        on_close=Callback::new(move |_| open.set(false))
                    />
                }
            }}
        </Portal>
    }
}

/* ---------------- 测量与排布 ---------------- */

/// 溢出菜单项：只包含当前被收起的动作，顺序沿用工具栏里的原始顺序。
/// 做成自由函数而不是闭包：`Portal` 的 children 必须是 `Fn`，
/// 闭包一旦把捕获的 Vec move 进去就只能调一次。
fn menu_items(actions: &[ToolbarAction], hidden: &[String]) -> Vec<MenuItem> {
    actions
        .iter()
        .filter(|a| hidden.iter().any(|h| h == &a.id))
        .map(|a| {
            let item = MenuItem::new(a.id.clone(), a.label.clone()).icon(a.icon);
            if a.disabled {
                item.disabled()
            } else {
                item
            }
        })
        .collect()
}

fn attr_id(el: &web_sys::Element) -> Option<String> {
    el.get_attribute("data-oa-id")
}

/// 工具栏上的动作按钮（不含「更多」）。
fn item_elements(root: &web_sys::HtmlElement) -> Vec<web_sys::Element> {
    let mut out = Vec::new();
    let Ok(list) = root.query_selector_all("[data-oa-id]") else {
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

/// 可聚焦且看得见的按钮（含「更多」），DOM 顺序。
fn focusable_buttons(root: &web_sys::HtmlElement) -> Vec<web_sys::HtmlElement> {
    let mut out = Vec::new();
    let Ok(list) = root.query_selector_all("button:not([hidden]):not([disabled])") else {
        return out;
    };
    for i in 0..list.length() {
        if let Some(node) = list.item(i) {
            if let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() {
                out.push(el);
            }
        }
    }
    out
}

/// 收纳顺序：`priority` 点过名的先走（列表顺序 = 被收顺序），剩下的从尾部往前。
fn removal_order(ids: &[String], priority: &[String]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::with_capacity(ids.len());
    for id in priority {
        if let Some(i) = ids.iter().position(|v| v == id) {
            if !out.contains(&i) {
                out.push(i);
            }
        }
    }
    // 右端本来就是次要动作，左端往往是「格式」「撤销」这类肌肉记忆位。
    for i in (0..ids.len()).rev() {
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

/// 一次完整排布：测量 → 决策 → 落 DOM，返回被收起的 id（DOM 顺序）。
///
/// 公式：`可用宽 = 容器 clientWidth - (「更多」宽 + gap)`，
/// 然后按收纳顺序逐个移出，直到「剩余项宽 + 间距」不超过可用宽（或触到 `min_visible`）。
fn layout(
    root: &web_sys::HtmlElement,
    more: &web_sys::HtmlElement,
    priority: &[String],
    gap: f64,
    min_visible: usize,
) -> Vec<String> {
    let items = item_elements(root);
    let ids: Vec<String> = items.iter().filter_map(attr_id).collect();
    if ids.len() != items.len() {
        // 结构对不上（有人在按钮里又套了 [data-oa-id]）：不动，保持现状。
        return Vec::new();
    }

    // ① 测量态：全部显形，容器只 visibility:hidden —— 量得到自然宽度，用户也看不见。
    let _ = root.set_attribute("data-measuring", "1");
    let _ = more.remove_attribute("hidden");
    for el in &items {
        let _ = el.remove_attribute("hidden");
    }

    let avail = root.client_width() as f64;
    let widths: Vec<f64> = items
        .iter()
        .map(|el| el.get_bounding_client_rect().width())
        .collect();
    let more_w = more.get_bounding_client_rect().width();

    // 量不到就不排：`display:none` 的祖先里一切宽度都是 0，
    // 这时候「全收进菜单」是最糟的答案 —— 整排显示，等下一次 resize 再说。
    if avail <= 0.0 || more_w <= 0.0 || widths.iter().all(|w| *w <= 0.0) {
        let _ = root.remove_attribute("data-measuring");
        let _ = more.set_attribute("hidden", "");
        return Vec::new();
    }

    // 预算先扣掉「更多」与它前面的间距：留出这个缝，临界宽度上不会
    // 出现「收起后刚好又放得下」的抖动。
    let budget = (avail - more_w - gap).max(0.0);
    let needed = |keep: &[usize]| -> f64 {
        keep.iter().map(|&i| widths[i]).sum::<f64>()
            + gap * (keep.len().saturating_sub(1)) as f64
    };

    let mut keep: Vec<usize> = (0..ids.len()).collect();
    if needed(&keep) > budget {
        for idx in removal_order(&ids, priority) {
            if keep.len() <= min_visible || needed(&keep) <= budget {
                break;
            }
            keep.retain(|&i| i != idx);
        }
    }

    let hidden: Vec<String> = ids
        .iter()
        .enumerate()
        .filter(|(i, _)| !keep.contains(i))
        .map(|(_, id)| id.clone())
        .collect();

    // 焦点所在的项马上要被藏起来：先把焦点交给「更多」。
    // 用 closet 到的 id 判断，而不是查 activeElement 是否已失效 —— 后者
    // 在 hidden 生效的瞬间就会变成 body。
    let focused_id = active_element().and_then(|el| {
        el.closest("[data-oa-id]")
            .ok()
            .flatten()
            .and_then(|e| attr_id(&e))
    });

    // ② 落结果：先设 hidden，最后摘测量态 —— 整个过程只触发一次样式重算。
    for (el, id) in items.iter().zip(ids.iter()) {
        if hidden.iter().any(|h| h == id) {
            let _ = el.set_attribute("hidden", "");
        }
    }
    if hidden.is_empty() {
        let _ = more.set_attribute("hidden", "");
    }
    let _ = root.remove_attribute("data-measuring");

    if let Some(fid) = focused_id {
        if hidden.iter().any(|h| h == &fid) {
            let _ = more.focus();
        }
    }
    hidden
}

/* ---------------- 尺寸监听 ---------------- */

/// 容器外层宽度：优先取父元素的客户端宽度（布局上游给的数，不由本组件内容决定），
/// 没有父级时退回自身。只用于「外部尺寸是否变了」的判断，不参与排布。
fn outer_width(root: &web_sys::HtmlElement) -> f64 {
    root.parent_element()
        .map(|p| p.client_width() as f64)
        .filter(|w| *w > 0.0)
        .unwrap_or_else(|| root.client_width() as f64)
}

/// `ResizeObserver` 的薄封装。
///
/// 为什么不用 web-sys 的绑定：`控件/Cargo.toml` 的 web-sys 特性表是整库共享的，
/// 这一批不允许改它；`ResizeObserver` 只在这一个组件里用，用 `Reflect` 取构造器
/// 比给整库加一个 feature 的改动面更小。环境不支持时（老 WebView）返回 `None`，
/// 组件仍靠 `window.resize` 工作。
struct SizeObserver {
    observer: JsValue,
    // 必须活到 disconnect：回调是 JS 侧持有的，提前回收会变成空指针。
    _cb: Closure<dyn FnMut(js_sys::Array, JsValue)>,
}

impl SizeObserver {
    fn new(mut on_resize: impl FnMut() + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let ctor = js_sys::Reflect::get(&window, &JsValue::from_str("ResizeObserver")).ok()?;
        let ctor: js_sys::Function = ctor.dyn_into().ok()?;
        let cb = Closure::wrap(
            Box::new(move |_entries: js_sys::Array, _observer: JsValue| on_resize())
                as Box<dyn FnMut(js_sys::Array, JsValue)>,
        );
        let args = js_sys::Array::of1(cb.as_ref().unchecked_ref());
        let observer = js_sys::Reflect::construct(&ctor, &args).ok()?;
        Some(Self {
            observer,
            _cb: cb,
        })
    }

    fn observe(&self, el: &web_sys::Element) {
        let Some(f) = js_sys::Reflect::get(&self.observer, &JsValue::from_str("observe"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
        else {
            return;
        };
        let _ = f.call1(&self.observer, el);
    }
}

impl Drop for SizeObserver {
    fn drop(&mut self) {
        let Some(f) = js_sys::Reflect::get(&self.observer, &JsValue::from_str("disconnect"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
        else {
            return;
        };
        let _ = f.call0(&self.observer);
    }
}

/// 装一次观察器：同时盯自身与父级。
/// 盯自身能收到「直接给宽度」的变化；盯父级能收到「父级变宽、自己还没长开」的变化
/// —— 隐藏项之后自身宽度停在内容宽，单看自己永远等不到变宽。
fn install_observer<F>(
    slot: &StoredValue<Option<SizeObserver>, LocalStorage>,
    root_ref: NodeRef<html::Div>,
    relayout: Rc<F>,
    outer: &StoredValue<f64, LocalStorage>,
) where
    F: Fn() + 'static,
{
    if slot.with_value(|o| o.is_some()) {
        return;
    }
    let Some(root) = root_ref.get_untracked() else {
        return;
    };
    let outer = *outer;
    let Some(observer) = SizeObserver::new(move || {
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        let now = outer_width(&root);
        // 父级宽度没变，说明这次尺寸变化是我们自己刚藏/放按钮的回声，
        // 不是外面真的变了 —— 再排一次只会得到同一个答案，还会抖。
        if (now - outer.get_value()).abs() < 1.0 {
            return;
        }
        outer.set_value(now);
        let relayout = relayout.clone();
        schedule(move || relayout());
    }) else {
        return;
    };
    observer.observe(&root);
    if let Some(parent) = root.parent_element() {
        observer.observe(&parent);
    }
    slot.set_value(Some(observer));
}
