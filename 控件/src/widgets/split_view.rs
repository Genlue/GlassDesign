//! 分栏：一条可拖的分隔条，两侧按比例分配主轴空间。
//!
//! 和旧库（`panels/split-view.js`，像素级约束求解 + 测量）的差别是刻意的：
//! 这里**不测量**。Rust 只往根节点写一个 `--split`（0..1），两栏宽度交给 CSS：
//!
//! ```css
//! .c-split__pane { flex: 0 0 var(--pane-size, 0px) }
//! ```
//!
//! 于是拖动时改的是比例，不是像素；容器换宽度（窗口缩放、侧栏展开）两栏一起
//! 跟着缩放，不需要 ResizeObserver，也不会出现「先量后改」那一帧的跳动。
//! 代价是：拖过分隔条本身只能按「比例」定，绝对像素宽度的分栏要用旧库那套测量，
//! 不在本次范围内。
//!
//! 三处边界：
//! 1. 只支持两栏。旧库的多栏 + 每栏独立 min/max 需要约束求解器，
//!    拆成两栏的嵌套反而更简单（嵌套两次就是三栏）。
//! 2. 折叠只折叠第一块（左 / 上）。折叠后根节点把 `--split` 写 0，
//!    另一栏自然吃满；分隔条原地不动，展开按钮就在它上面 —— 这是唯一
//!    「栏没了还抓得住」的位置。
//! 3. `children` 拆成两个槽（`first` / `second`）而不是一个：Leptos 的
//!    `Children` 是 `FnOnce`，一份 children 无法拆给两个 pane。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};

/// 分隔条的命中区宽度，必须与 `.c-split__sep { flex: 0 0 9px }` 一致。
/// 拖动时把指针位移换算成比例要扣掉它，否则两栏永远差 9px 铺不满。
const SEP_PX: f64 = 9.0;
/// 键盘步进：2% / Shift 10%，和系统分隔条的手感对齐。
const STEP: f64 = 0.02;
const STEP_BIG: f64 = 0.10;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SplitOrientation {
    /// 左右分栏（分隔条竖着）
    #[default]
    Horizontal,
    /// 上下分栏（分隔条横着）
    Vertical,
}

/// 拖动期间接管整页光标。`html[data-resizing]` 的规则在 43-resizer.css 里，
/// 和独立的 `Resizer` 共用同一套，不必再写一份。
fn set_resizing(axis: Option<&str>) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(root) = doc.document_element() else {
        return;
    };
    match axis {
        Some(a) => {
            let _ = root.set_attribute("data-resizing", a);
        }
        None => {
            let _ = root.remove_attribute("data-resizing");
        }
    }
}

#[component]
pub fn SplitView(
    #[prop(optional)] orientation: SplitOrientation,
    /// 初值 / 双击复位的比例（0..1，第一块占主轴的份额）。
    #[prop(default = 0.3)] initial: f64,
    /// 第一块的下限比例。
    #[prop(default = 0.15)] min: f64,
    /// 第一块的上限比例。
    #[prop(default = 0.85)] max: f64,
    /// 是否给第一块一个折叠入口（分隔条上的那颗按钮）。
    #[prop(optional)] collapsible: bool,
    /// 第一块：左 / 上。
    first: Children,
    /// 第二块：右 / 下。
    second: Children,
) -> impl IntoView {
    let vertical = orientation == SplitOrientation::Vertical;
    let (lo, hi) = {
        let a = if min.is_finite() { min.clamp(0.0, 1.0) } else { 0.15 };
        let b = if max.is_finite() { max.clamp(0.0, 1.0) } else { 0.85 };
        // 上下限写反或贴死时退回一个能拖的区间，不要留一条拖不动的缝。
        if b - a < 0.02 {
            (0.1, 0.9)
        } else {
            (a, b)
        }
    };
    let home = if initial.is_finite() { initial.clamp(lo, hi) } else { lo };
    let split = RwSignal::new(home);
    let dragging = RwSignal::new(false);
    let collapsed = RwSignal::new(false);
    // (按下时的指针坐标, 按下时的比例)：拖动全程用增量算，避免累计误差。
    let drag = RwSignal::new(None::<(f64, f64)>);

    let root_ref = NodeRef::<html::Div>::new();

    // 百分比布局不需要测量，只有「指针位移 → 比例」这一步要知道主轴长度，
    // 所以只在拖动中读一次 rect，渲染期完全不碰布局。
    let extent = move || {
        let Some(el) = root_ref.get_untracked() else { return 0.0 };
        let r = el.get_bounding_client_rect();
        let total = if vertical { r.height() } else { r.width() };
        (total - SEP_PX).max(1.0)
    };

    let axis = if vertical { "y" } else { "x" };
    // 水平分栏的分隔条是竖的，aria-orientation 描述的是分隔条本身，不是栏的排列。
    let sep_orientation = if vertical { "horizontal" } else { "vertical" };
    // 折叠后写 0：第一块 `display:none`，第二块自然吃满剩余空间，
    // 不需要额外的 CSS 分支去判断「谁被折叠了」。
    let css_split = move || if collapsed.get() { 0.0 } else { split.get() };
    let pct = move || (split.get() * 100.0).round();

    let collapse_icon = if vertical { IconName::CHEVRON_UP } else { IconName::CHEVRON_LEFT };
    let expand_icon = if vertical { IconName::CHEVRON_DOWN } else { IconName::CHEVRON_RIGHT };

    view! {
        <div
            class="c-split"
            data-axis=axis
            data-ratio="1"
            role="group"
            aria-label="分栏"
            node_ref=root_ref
            style=move || format!("--split:{:.4}", css_split())
        >
            <div
                class="c-split__pane"
                data-slot="a"
                data-collapsed=move || collapsed.get().then_some("1")
            >
                {first()}
            </div>
            <div
                class="c-split__sep"
                role="separator"
                tabindex="0"
                data-active=move || dragging.get().then_some("1")
                aria-orientation=sep_orientation
                aria-label="调整分栏比例"
                aria-valuemin=(lo * 100.0).round().to_string()
                aria-valuemax=(hi * 100.0).round().to_string()
                aria-valuenow=move || pct().to_string()
                aria-valuetext=move || format!("第一栏 {}%", pct())
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    ev.prevent_default();
                    if let Some(target) = ev.current_target() {
                        if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                            let _ = el.set_pointer_capture(ev.pointer_id());
                        }
                    }
                    let at = if vertical { ev.client_y() as f64 } else { ev.client_x() as f64 };
                    drag.set(Some((at, split.get_untracked())));
                    dragging.set(true);
                    set_resizing(Some(axis));
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    let Some((start, base)) = drag.get_untracked() else { return };
                    let span = extent();
                    if span <= 0.0 {
                        return;
                    }
                    let at = if vertical { ev.client_y() as f64 } else { ev.client_x() as f64 };
                    split.set((base + (at - start) / span).clamp(lo, hi));
                }
                on:pointerup=move |ev: web_sys::PointerEvent| {
                    if drag.get_untracked().is_none() {
                        return;
                    }
                    drag.set(None);
                    dragging.set(false);
                    set_resizing(None);
                    if let Some(target) = ev.current_target() {
                        if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                            let _ = el.release_pointer_capture(ev.pointer_id());
                        }
                    }
                }
                on:pointercancel=move |_| {
                    drag.set(None);
                    dragging.set(false);
                    set_resizing(None);
                }
                on:dblclick=move |_| {
                    // 复位到调用方给的初值，不是「一半一半」——
                    // 等分会把 30/70 这类有意的结构一夜抹平。
                    collapsed.set(false);
                    split.set(home);
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.is_composing() {
                        return;
                    }
                    let step = if ev.shift_key() { STEP_BIG } else { STEP };
                    let (back, fwd) = if vertical {
                        ("ArrowUp", "ArrowDown")
                    } else {
                        ("ArrowLeft", "ArrowRight")
                    };
                    let key = ev.key();
                    let next = match key.as_str() {
                        k if k == back => split.get_untracked() - step,
                        k if k == fwd => split.get_untracked() + step,
                        "Home" => lo,
                        "End" => hi,
                        _ => return,
                    };
                    ev.prevent_default();
                    // 折叠状态下方向键先把栏打开，否则只是在一条空缝上挪数字。
                    collapsed.set(false);
                    split.set(next.clamp(lo, hi));
                }
            >
                {collapsible.then(|| {
                    view! {
                        <button
                            class="c-split__expand"
                            type="button"
                            // 展开态把按钮降级成「悬停才出现」：它压在 9px 命中区正中，
                            // 常驻会挡掉最顺手的拖动位置。
                            data-idle=move || (!collapsed.get()).then_some("1")
                            aria-expanded=move || (!collapsed.get()).to_string()
                            aria-label=move || {
                                if collapsed.get() { "展开第一栏" } else { "折叠第一栏" }
                            }
                            on:pointerdown=move |ev: web_sys::PointerEvent| {
                                // 别让按按钮变成开始拖动。
                                ev.stop_propagation();
                            }
                            on:click=move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                collapsed.update(|c| *c = !*c);
                            }
                        >
                            <span class="c-split__expand-icon">
                                {move || {
                                    let name = if collapsed.get() { expand_icon } else { collapse_icon };
                                    view! { <Icon name=name size=12 /> }
                                }}
                            </span>
                        </button>
                    }
                })}
            </div>
            <div class="c-split__pane" data-slot="b">{second()}</div>
        </div>
    }
}
