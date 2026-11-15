use std::cell::Cell;
use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::base::anchor::{self, AnchorSpec, AnchorWatch, Placement, Side};
use crate::base::id::next_id;
use crate::base::overlay::{
    first_focusable, listen_document, listen_window, schedule, GlobalListener, OverlayKind,
};

/// 提示出现的方向。`Bottom` 是默认：图标按钮上方通常有内容，往下展开更少遮挡。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum TooltipPlacement {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

fn viewport() -> (f64, f64) {
    let Some(w) = web_sys::window() else {
        return (1280.0, 800.0);
    };
    let width = w
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let height = w
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    (width, height)
}

/// 量提示的内容高度：不受当前 `max-height` 影响（气泡不滚动，但共享层在
/// `scroll=false` 时仍可能写 `max-height`）。
fn content_height(panel: &web_sys::HtmlElement) -> f64 {
    let style = panel.style();
    let prev = style.get_property_value("max-height").unwrap_or_default();
    let _ = style.set_property("max-height", "none");
    let h = panel.scroll_height() as f64;
    let _ = style.set_property("max-height", &prev);
    h
}

fn clamp_axis(value: f64, size: f64, view: f64, margin: f64) -> f64 {
    let max = (view - size - margin).max(margin);
    value.clamp(margin, max)
}

/// 按 `TooltipPlacement` 算四向位置：居中贴边，越界翻到另一侧，最后夹进视口。
fn place_tooltip(
    anchor_el: &web_sys::Element,
    panel: &web_sys::HtmlElement,
    spec: AnchorSpec,
    placement: TooltipPlacement,
) -> Placement {
    let a = anchor_el.get_bounding_client_rect();
    let (vw, vh) = viewport();
    let margin = spec.margin;
    let width = panel.offset_width() as f64;
    let height = content_height(panel);

    let (left, mut top, mut side) = match placement {
        TooltipPlacement::Bottom => (
            a.left() + a.width() / 2.0 - width / 2.0,
            a.bottom() + spec.gap,
            Side::Below,
        ),
        TooltipPlacement::Top => (
            a.left() + a.width() / 2.0 - width / 2.0,
            a.top() - height - spec.gap,
            Side::Above,
        ),
        TooltipPlacement::Left => (
            a.left() - width - spec.gap,
            a.top() + a.height() / 2.0 - height / 2.0,
            Side::Below,
        ),
        TooltipPlacement::Right => (
            a.right() + spec.gap,
            a.top() + a.height() / 2.0 - height / 2.0,
            Side::Below,
        ),
    };

    // 底部越界翻上（默认方向），顶部越界翻下；横向永远夹取。
    match placement {
        TooltipPlacement::Bottom if top + height > vh - margin => {
            top = a.top() - height - spec.gap;
            side = Side::Above;
        }
        TooltipPlacement::Top if top < margin => {
            top = a.bottom() + spec.gap;
            side = Side::Below;
        }
        _ => {}
    }

    // 左右侧向没有「翻」的语义，但上下仍要夹住，避免贴边时半截在视口外。
    if matches!(placement, TooltipPlacement::Left | TooltipPlacement::Right) {
        side = if top >= a.top() {
            Side::Below
        } else {
            Side::Above
        };
    }

    Placement {
        side,
        left: clamp_axis(left, width, vw, margin),
        top: clamp_axis(top, height, vh, margin),
        width,
        max_height: f64::INFINITY,
    }
}

/// 面板尺寸变化的观察者（`ResizeObserver` 的最小封装）。
struct PanelObserver {
    observer: JsValue,
    closure: Closure<dyn FnMut()>,
}

impl PanelObserver {
    fn new(target: web_sys::HtmlElement, on_change: impl FnMut() + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let ctor = js_sys::Reflect::get(&window, &JsValue::from_str("ResizeObserver")).ok()?;
        if !ctor.is_function() {
            return None;
        }
        let ctor: js_sys::Function = ctor.unchecked_into();
        let closure = Closure::wrap(Box::new(on_change) as Box<dyn FnMut()>);
        let observer = js_sys::Reflect::construct(
            &ctor,
            &js_sys::Array::of1(closure.as_ref().unchecked_ref()),
        )
        .ok()?;
        let observe: js_sys::Function =
            js_sys::Reflect::get(&observer, &JsValue::from_str("observe")).ok()?.unchecked_into();
        let _ = observe.call1(&observer, target.unchecked_ref());
        Some(Self { observer, closure })
    }
}

impl Drop for PanelObserver {
    fn drop(&mut self) {
        if let Ok(disconnect) = js_sys::Reflect::get(&self.observer, &JsValue::from_str("disconnect")) {
            if let Ok(f) = disconnect.dyn_into::<js_sys::Function>() {
                let _ = f.call0(&self.observer);
            }
        }
        let _ = &self.closure;
    }
}

/// 共享 `anchor::watch` 只认「贴下方/翻上方 + 左对齐」，提示需要居中/侧向，
/// 因此在这条跟随链路之后再补一次对齐：监听器按注册顺序在同一个 rAF 里跑。
struct AlignFix {
    _scroll: Option<GlobalListener>,
    _resize: Option<GlobalListener>,
    _observer: Option<PanelObserver>,
}

impl AlignFix {
    fn new(
        anchor_el: web_sys::Element,
        panel: web_sys::HtmlElement,
        apply: Rc<dyn Fn(&web_sys::Element, &web_sys::HtmlElement)>,
    ) -> Self {
        let queued = Rc::new(Cell::new(false));
        let pump: Rc<dyn Fn()> = {
            let anchor_el = anchor_el.clone();
            let panel = panel.clone();
            let queued = Rc::clone(&queued);
            let apply = Rc::clone(&apply);
            Rc::new(move || {
                if queued.replace(true) {
                    return;
                }
                let anchor_el = anchor_el.clone();
                let panel = panel.clone();
                let queued = Rc::clone(&queued);
                let apply = Rc::clone(&apply);
                schedule(move || {
                    queued.set(false);
                    if !anchor_el.is_connected() || !panel.is_connected() {
                        return;
                    }
                    apply(&anchor_el, &panel);
                });
            })
        };

        let scroll = listen_window("scroll", true, {
            let pump = Rc::clone(&pump);
            move |_| pump()
        });
        let resize = listen_window("resize", false, {
            let pump = Rc::clone(&pump);
            move |_| pump()
        });
        let observer = PanelObserver::new(panel, {
            let pump = Rc::clone(&pump);
            move || pump()
        });

        Self {
            _scroll: scroll,
            _resize: resize,
            _observer: observer,
        }
    }
}

struct TipFollow {
    _anchor: AnchorWatch,
    _fix: AlignFix,
}

/// 悬停提示。只给「没有可见文字」的控件用 —— 有文字的控件加提示是噪音。
///
/// 三条规则：
/// 1) 不抢焦点、不参与 Tab 序列：提示是补充说明，不是可以进去操作的内容。
/// 2) 触发器用 `aria-describedby` 关联到提示节点，而不是 `aria-label` ——
///    后者会替换掉按钮自己的名字。
/// 3) hover 等 `delay`，focus 立即出：键盘用户已经在目标上了，再等就是惩罚。
#[component]
pub fn Tooltip(
    #[prop(into)] label: String,
    /// 悬停延迟（毫秒）。0 走默认 400。
    #[prop(optional)] delay: u32,
    #[prop(optional)] placement: TooltipPlacement,
    children: Children,
) -> impl IntoView {
    let wrap_ref = NodeRef::<html::Span>::new();
    let tip_ref = NodeRef::<html::Div>::new();
    let tip_id = next_id("gd-tip");
    let tip_id_key = StoredValue::new(tip_id);
    let label_key = StoredValue::new(label);
    let shown = RwSignal::new(false);
    // 提示不进 Esc 栈：它是弱交互，Esc 不关（只由离开 / 失焦 / 点外收起）。
    let open = RwSignal::new(false);
    let watch = StoredValue::new_local(None::<TipFollow>);
    let timer = StoredValue::new_local(None::<i32>);
    let described = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let press_listener = StoredValue::new_local(None::<GlobalListener>);
    let blur_listener = StoredValue::new_local(None::<GlobalListener>);
    let delay = if delay == 0 { 400 } else { delay };
    // z 取提示层的基线：没有入栈，所以不会随别的浮层深度变化。
    let tip_z = OverlayKind::Tooltip.base_z();

    let hide = move || {
        if let Some(id) = timer.get_value() {
            if let Some(w) = web_sys::window() {
                w.clear_timeout_with_handle(id);
            }
            timer.set_value(None);
        }
        shown.set(false);
        open.set(false);
        watch.set_value(None);
        // 收起后重新藏到视口外：提示常驻 body，不能让「已收起的提示」留在原位
        // 只靠 opacity 隐身（读屏 / 断言仍会看到一个 visible 的节点）。
        if let Some(tip) = tip_ref.get() {
            anchor::hide_until_placed(&tip);
        }
        if let Some(el) = described.get_value() {
            let _ = el.remove_attribute("aria-describedby");
            described.set_value(None);
        }
    };

    let show = move || {
        if open.get_untracked() {
            return;
        }
        let Some(wrap) = wrap_ref.get() else { return };
        let anchor_el: web_sys::Element = wrap.clone().into();
        open.set(true);
        // 关联关系要挂在真正能聚焦的那个元素上：包装器只是用来测量的 span。
        let target = first_focusable(&wrap).unwrap_or_else(|| wrap.clone().into());
        let _ = target.set_attribute("aria-describedby", &tip_id_key.get_value());
        described.set_value(Some(target));

        // 共享层：气泡档（不滚动、不跟锚点同宽）。提示自己按方向算位置后
        // 仍然走这条 watch —— 锚点滚出视口时 `on_gone=None`，让它跟着走而不是弹关闭。
        let spec = AnchorSpec {
            scroll: false,
            max_height: f64::INFINITY,
            ..AnchorSpec::bubble()
        };
        schedule(move || {
            if !open.get_untracked() {
                return;
            }
            let Some(tip) = tip_ref.get() else {
                // Portal 还没挂好（极少见）：放开状态，下一次 hover / focus 再试。
                open.set(false);
                return;
            };
            if !anchor_el.is_connected() || !tip.is_connected() {
                open.set(false);
                return;
            }
            let placed = place_tooltip(&anchor_el, &tip, spec, placement);
            anchor::apply(&tip, placed, spec);
            let tip_el: web_sys::HtmlElement = tip.into();

            // 先挂共享 watch，再挂 AlignFix：监听器按注册顺序执行，后注册的这一份
            // 才能覆盖共享层的「贴下方 + 左对齐」。
            let anchor_watch = anchor::watch(anchor_el.clone(), tip_el.clone(), spec, None);
            let apply = Rc::new(move |a: &web_sys::Element, p: &web_sys::HtmlElement| {
                if !open.get_untracked() {
                    return;
                }
                let placed = place_tooltip(a, p, spec, placement);
                anchor::apply(p, placed, spec);
            });
            let fix = AlignFix::new(anchor_el, tip_el, apply);
            watch.set_value(Some(TipFollow {
                _anchor: anchor_watch,
                _fix: fix,
            }));

            // 下一帧再打开过渡：同一帧里改 left/top 与 opacity 时，
            // 浏览器会把位移一起算进 transition，看起来像从旧位置飘过来。
            schedule(move || {
                if open.get_untracked() {
                    shown.set(true);
                }
            });
        });
    };

    let hover_in = move |_: web_sys::PointerEvent| {
        if let Some(id) = timer.get_value() {
            if let Some(w) = web_sys::window() {
                w.clear_timeout_with_handle(id);
            }
        }
        let cb = wasm_bindgen::closure::Closure::once_into_js(move || show());
        if let Some(w) = web_sys::window() {
            if let Ok(id) = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.unchecked_ref::<js_sys::Function>(),
                delay as i32,
            ) {
                timer.set_value(Some(id));
            }
        }
    };

    let focus_in = move |_: web_sys::FocusEvent| show();

    let focus_out = move |ev: web_sys::FocusEvent| {
        // 焦点在包装器内部移动（例如触发器里有多个按钮）不该收起。
        let inside = ev
            .related_target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .zip(wrap_ref.get())
            .map(|(node, wrap)| wrap.contains(Some(&node)))
            .unwrap_or(false);
        if !inside {
            hide();
        }
    };

    // 失焦 / 任意点击都收起：提示不能卡在屏幕中间。
    // 滚动不再直接收起 —— 定位已由 anchor 的跟随链路接管，锚点滚出视口时
    // `on_gone=None` 会让它跟着走，而不是在滚动一发生就消失。
    if let Some(l) = listen_document("pointerdown", move |_| hide()) {
        press_listener.set_value(Some(l));
    }
    if let Some(l) = listen_window("blur", false, move |_| hide()) {
        blur_listener.set_value(Some(l));
    }

    on_cleanup(move || {
        hide();
        press_listener.set_value(None);
        blur_listener.set_value(None);
    });

    view! {
        <span
            style="display:inline-flex"
            node_ref=wrap_ref
            on:pointerenter=hover_in
            on:pointerleave=move |_| hide()
            on:focusin=focus_in
            on:focusout=focus_out
        >
            {children()}
        </span>

        // 提示也必须到 body：祖先的 transform / backdrop-filter 会让 fixed 以它为包含块。
        <Portal>
            <div
                class="c-tip"
                role="tooltip"
                id=tip_id_key.get_value()
                data-show=move || shown.get().then_some("1")
                node_ref=tip_ref
                // 定稿前一直藏在视口外；`anchor::apply` 逐条写内联样式并显形。
                // 这条闭包不依赖任何信号，定位后不会再把已写好的内联样式整条覆盖掉。
                style=move || {
                    format!("z-index:{tip_z};position:fixed;left:-10000px;top:0px;visibility:hidden")
                }
            >
                {label_key.get_value()}
            </div>
        </Portal>
    }
}
