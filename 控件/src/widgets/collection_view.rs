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
//! 3) 变高（`virtual_var_height` + `max_height` + 列表布局）：行高由内容决定，
//!    用 `base/virtual_window::Heights` 的前缀和表算窗口，上下撑高行给**像素**
//!    而不是「行数 × 行高」。高度先按 `item_height` 估、挂载后用 `ResizeObserver`
//!    实测校正（差值 > 0.5px 才写回，否则会喂出「量 → 改表 → 换窗口 → 再量」的
//!    死循环）。网格布局不接：它的「行」是隐式的（一行 `cols` 项、行高取最高的
//!    那一项），前缀和记的是「项」不是「行」，先拍平成行是另一套东西。
//!
//! 选择与键盘一律按**全量下标**算（`order` / `item_count`），渲染窗口只影响
//! 「DOM 里有哪些项」，不影响状态机。

use std::ops::Range;
use std::rc::Rc;

use leptos::html;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::base::i18n::t;
use crate::base::virtual_window::{self, Heights, VarWindow, Window};
use crate::widgets::selection::{SelectionMode, SelectionState};

/// 列表项之间的间距（px）。CSS 里是 `--s-2`：变高模式的撑高行要减掉紧跟在自己
/// 身后的那个 gap，所以 Rust 也得知道这个数 —— 两边必须是同一个数（红线 5）。
const ROW_GAP: f64 = 4.0;

/// 实测高度与表里的值差多少才写回。亚像素抖动（0.5px 以内）不写，
/// 否则「量 → 改表 → 换窗口 → DOM 变 → 再量」会变成一帧一帧的死循环。
const HEIGHT_EPS: f64 = 0.5;

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

/// 取 JS 对象上的一个方法。
fn callable(target: &JsValue, name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(target, &JsValue::from_str(name))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()
}

/// 「刚好让它可见」地滚一次。
///
/// 键盘移动用：`ensure_row` 只保证这一行的**顶边**落进视口，变高项可能比视口还高，
/// 底部会被切掉。补一次 `scrollIntoView({block:'nearest'})`：元素本来就在视口里
/// 时浏览器什么都不做，所以不会和 `ensure_row` 抢滚动位置。
fn scroll_into_view_nearest(el: &web_sys::HtmlElement) {
    let Some(f) = callable(el.as_ref(), "scrollIntoView") else {
        return;
    };
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("block"), &JsValue::from_str("nearest"));
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("inline"),
        &JsValue::from_str("nearest"),
    );
    let _ = f.call1(el.as_ref(), opts.as_ref());
}

/// 项高度变化的观察器（`ResizeObserver` 的最小封装）。
///
/// 不用 web-sys 的绑定：`控件/Cargo.toml` 的 web-sys 特性表是整库共享的，
/// 为一个组件加 `ResizeObserver` feature 的改动面比 `Reflect` 大得多。
/// 环境不支持时返回 `None`，组件退化成「只在窗口变化时量一次」，仍能工作。
struct RowObserver {
    observer: JsValue,
    // 必须活到 disconnect：回调由 JS 侧持有，提前回收会变成空指针。
    _cb: Closure<dyn FnMut(js_sys::Array, JsValue)>,
}

impl RowObserver {
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

    /// 换一批观察目标：先 disconnect 再 observe。
    ///
    /// 只 observe 不 disconnect 的话，滚一遍整表就把所有已经摘掉的节点都留在
    /// 观察器里（它会强引用目标）；变高列表滚过 600 项就是 600 个僵尸节点。
    fn retarget<'a>(&self, els: impl Iterator<Item = &'a web_sys::HtmlElement>) {
        let observer = &self.observer;
        if let Some(f) = callable(observer, "disconnect") {
            let _ = f.call0(observer);
        }
        let Some(observe) = callable(observer, "observe") else {
            return;
        };
        for el in els {
            let _ = observe.call1(observer, el.as_ref());
        }
    }
}

impl Drop for RowObserver {
    fn drop(&mut self) {
        if let Some(f) = callable(&self.observer, "disconnect") {
            let _ = f.call0(&self.observer);
        }
    }
}

/// 把观察器挂到「当前窗口里的项」上。观察器只建一次，之后每次换窗口重挂目标。
fn attach_rows(
    slot: StoredValue<Option<RowObserver>, LocalStorage>,
    refs: &[NodeRef<html::Div>],
    range: Range<usize>,
    measure: Rc<dyn Fn() -> bool>,
    recompute: Rc<dyn Fn()>,
) {
    let targets: Vec<web_sys::HtmlElement> = range.filter_map(|row| live(refs.get(row))).collect();
    if slot.with_value(|o| o.is_some()) {
        slot.with_value(|o| {
            if let Some(obs) = o {
                obs.retarget(targets.iter());
            }
        });
        return;
    }
    let Some(obs) = RowObserver::new(move || {
        if measure() {
            recompute();
        }
    }) else {
        return;
    };
    obs.retarget(targets.iter());
    slot.set_value(Some(obs));
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
    /// 变高行虚拟化：行高由内容决定，先估计再实测校正。只在**列表**布局生效，
    /// 且需要 `max_height > 0`；不传就是老的固定行高行为。
    #[prop(optional)] virtual_var_height: bool,
) -> impl IntoView {
    // 兜底查文案表，留在属性 / 文本闭包里（切语言要跟着变）。
    let label_store = StoredValue::new_local(label);
    let empty_store = StoredValue::new_local(empty_text);
    let label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().collection_label.to_string()
        } else {
            own
        }
    };
    let empty_text = move || {
        let own = empty_store.get_value();
        if own.is_empty() {
            t().empty_collection.to_string()
        } else {
            own
        }
    };
    let cols = if layout == CollectionLayout::Grid {
        columns.max(1) as usize
    } else {
        1
    };
    let item_h = if item_height > 0.0 {
        item_height.max(1.0)
    } else {
        32.0
    };

    // 变高只接列表：网格的「行」是隐式的（一行 cols 项、行高由最高的那一项决定），
    // 前缀和记的是「项」不是「行」，要接得先把网格拍平成行 —— 那是另一套东西。
    let var_mode = virtual_var_height && layout == CollectionLayout::List && max_height > 0.0;

    // 两个条件都满足才真正虚拟化：只给行距（或变高开关）没有视口，
    // 量不出「该渲染哪几行」。
    let pitch = virtual_row_height.unwrap_or(item_h).max(1.0);
    let virtual_mode = (virtual_row_height.is_some() || var_mode) && max_height > 0.0;

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

    // 变高：高度表 + 窗口都按**像素**算。估计值取 `item_height`（估计 ≤ 实际时
    // 首帧只会多渲染，不会露白）；overscan 是像素，不是行数。
    let overscan_px = item_h * virtual_window::OVERSCAN as f64;
    let heights = RwSignal::new(Heights::uniform(item_count, item_h));
    let vwin = RwSignal::new(if var_mode {
        heights
            .get_untracked()
            .window(0.0, max_height.max(1.0), overscan_px)
    } else {
        VarWindow::new(0, 0, 0.0, 0.0)
    });

    let all_items = items;
    let visible = Memo::new(move |_| {
        if var_mode {
            let w = vwin.get();
            let start = w.start().min(item_count);
            let end = w.end().min(item_count).max(start);
            return all_items[start..end]
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, item)| (start + i, item))
                .collect::<Vec<_>>();
        }
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

    // 量当前窗口里的项，写回高度表。只量已挂载且还在文档上的节点：NodeRef
    // 可能还指着上一屏被摘掉的元素。差值 ≤ 0.5px 不写 —— 亚像素抖动会喂出
    // 「量 → 改表 → 换窗口 → DOM 变 → 再量」的死循环。
    let refs_measure = refs.clone();
    let measure: Rc<dyn Fn() -> bool> = Rc::new(move || {
        let mut table = heights.get_untracked();
        if table.len() != item_count {
            // 项数变了（换了一批数据）：表跟着截断/重建，别留上一批的脏高度（坑 7）。
            if table.len() > item_count {
                table.truncate(item_count);
            } else {
                table = Heights::uniform(item_count, item_h);
            }
        }
        let mut changed = false;
        for row in vwin.get_untracked().slice() {
            let Some(el) = live(refs_measure.get(row)) else {
                continue;
            };
            let real = el.get_bounding_client_rect().height();
            if real > HEIGHT_EPS && (table.height(row) - real).abs() > HEIGHT_EPS {
                table.set(row, real);
                changed = true;
            }
        }
        if changed {
            heights.set(table);
        }
        changed
    });

    // 由「当前滚动位置 + 高度表」重算窗口。高度一改就得调一次，否则上下撑高行
    // 还会按旧的总高算，滚到底会少一截。
    let recompute: Rc<dyn Fn()> = Rc::new(move || {
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        let next = heights.get_untracked().window(
            root.scroll_top() as f64,
            root.client_height() as f64,
            overscan_px,
        );
        if vwin.get_untracked() != next {
            vwin.set(next);
        }
    });

    // max_height 只是上限，实际高度可能更小：挂载后按真实视口再量一次。
    let measured = RwSignal::new(false);
    {
        let measure = measure.clone();
        let recompute = recompute.clone();
        Effect::new(move |_| {
            if !virtual_mode || measured.get_untracked() {
                return;
            }
            measured.set(true);
            let (measure, recompute) = (measure.clone(), recompute.clone());
            schedule(move || {
                if var_mode {
                    // 先量再算：估计值换成一版真高，窗口才准。
                    measure();
                    recompute();
                    return;
                }
                if let Some(el) = root_ref.get_untracked() {
                    let next = virtual_window::window_of(
                        &el,
                        pitch,
                        total_rows,
                        virtual_window::OVERSCAN,
                    );
                    if win.get_untracked() != next {
                        win.set(next);
                    }
                }
            });
        });
    }

    // 变高专用：观察窗口内所有已挂载项。窗口一变就重挂目标 —— 观察器会强引用
    // 目标节点，只挂不摘的话滚一遍整表就把所有 DOM 都留住了。
    if var_mode {
        let watch = StoredValue::new_local(None::<RowObserver>);
        let (measure_ob, recompute_ob) = (measure.clone(), recompute.clone());
        let refs_ob = refs.clone();
        Effect::new(move |_| {
            let _ = vwin.get(); // 跟踪窗口：切片一变就换一批观察目标
            let (measure, recompute) = (measure_ob.clone(), recompute_ob.clone());
            let refs_ob = refs_ob.clone();
            schedule(move || {
                // 新挂载的项在这一帧才量得到；量到就立刻重算窗口
                if measure() {
                    recompute();
                }
                // 范围在 rAF 里重读：一帧之内窗口可能已经换过一次
                attach_rows(
                    watch,
                    &refs_ob,
                    vwin.get_untracked().slice(),
                    measure,
                    recompute,
                );
            });
        });
    }

    // 让第 `row` 行落进窗口：先滚（必要时），再按新的滚动位置重算窗口。
    // 用 `Rc` 而不是裸闭包：变高分支要抓 `recompute`，闭包就不再是 `Copy`，
    // 而 `move_focus` 与 `keydown` 都要调它。
    let recompute_row = recompute.clone();
    let ensure_row: Rc<dyn Fn(&web_sys::Element, usize)> =
        Rc::new(move |root: &web_sys::Element, row: usize| {
            let vh = root.client_height() as f64;
            let top = root.scroll_top() as f64;
            if var_mode {
                // 变高：位置查表（前缀和），不能用 row × 行高
                let table = heights.get_untracked();
                let y = table.offset(row);
                let h = table.height(row);
                let next = if y < top {
                    y
                } else if y + h > top + vh {
                    y + h - vh
                } else {
                    top
                };
                if next != top {
                    root.set_scroll_top(next.max(0.0).round() as i32);
                }
                recompute_row();
                return;
            }
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
        });

    let ensure_focus = ensure_row.clone();
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
                ensure_focus(root, next / cols);
            }
        }
        match live(refs_focus.get(next)) {
            Some(el) => {
                // 变高项可能比视口还高：`ensure_row` 只保证顶边进视口，底部仍可能
                // 被切掉。补一次 nearest（已经完整可见时浏览器不动），再
                // `preventScroll` 聚焦 —— 否则 `focus()` 会把目标居中（坑 6）。
                if var_mode {
                    scroll_into_view_nearest(&el);
                }
                focus_without_scroll(&el);
            }
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
                            if var_mode {
                                scroll_into_view_nearest(&el);
                            }
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
        if var_mode {
            // 变高：高度表可能刚被校正过，窗口一律按前缀和重算
            recompute();
        } else {
            let next =
                virtual_window::window_of(&root, pitch, total_rows, virtual_window::OVERSCAN);
            if win.get_untracked() != next {
                win.set(next);
            }
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
    let ensure_keys = ensure_row.clone();
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
            // 变高看自己那张表，固定行高看 `win`
            let in_window = if var_mode {
                vwin.get_untracked().contains(row)
            } else {
                clamp_win().contains(row)
            };
            if !in_window {
                if let Some(root) = root_ref.get_untracked() {
                    ensure_keys(&root, row);
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
            data-varheight=var_mode.then_some("1")
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
                        if var_mode {
                            // 变高：撑高行的高度是**像素**（前缀和）。再减掉身后那个
                            // gap，因为 grid 的 row-gap 只在两个子项之间出现。
                            view! {
                                <div
                                    class="c-collection__pad"
                                    role="presentation"
                                    aria-hidden="true"
                                    data-pad=move || {
                                        if vwin.get().pad_top() - ROW_GAP > HEIGHT_EPS {
                                            "1"
                                        } else {
                                            "0"
                                        }
                                    }
                                    style=move || {
                                        let pad = (vwin.get().pad_top() - ROW_GAP).max(0.0);
                                        format!("--pad-h:{pad}px")
                                    }
                                ></div>
                            }
                            .into_any()
                        } else {
                            view! {
                                <div
                                    class="c-collection__pad"
                                    role="presentation"
                                    aria-hidden="true"
                                    data-rows=move || clamp_win().start().to_string()
                                    style=move || format!("--rows:{}", clamp_win().start())
                                ></div>
                            }
                            .into_any()
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
                        if var_mode {
                            // 下撑高行不减 gap：它前面那个 gap 本来就在总高里，
                            // «留白 + gap» 正好补到 offset(end)（推导见 VarWindow）。
                            view! {
                                <div
                                    class="c-collection__pad"
                                    role="presentation"
                                    aria-hidden="true"
                                    data-pad=move || {
                                        if vwin.get().pad_bottom() > HEIGHT_EPS {
                                            "1"
                                        } else {
                                            "0"
                                        }
                                    }
                                    style=move || format!("--pad-h:{}px", vwin.get().pad_bottom())
                                ></div>
                            }
                            .into_any()
                        } else {
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
                            .into_any()
                        }
                    })}
                }
                .into_any()
            }}
        </div>
    }
}
