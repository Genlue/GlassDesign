use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::id::next_id;
use crate::base::overlay::{
    first_focusable, listen_document, listen_window, schedule, GlobalListener, OverlayHost,
    OverlayId, OverlayKind,
};

/// 与目标之间的距离。和菜单一样是定位几何，不是视觉令牌。
const GAP: f64 = 6.0;
const EDGE: f64 = 6.0;

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
    let host = OverlayHost::use_or_provide();
    let wrap_ref = NodeRef::<html::Span>::new();
    let tip_ref = NodeRef::<html::Div>::new();
    let tip_id = next_id("gd-tip");
    let tip_id_attr = tip_id.clone();
    let tip_id_key = StoredValue::new(tip_id);
    let shown = RwSignal::new(false);
    let pos = RwSignal::new((0.0f64, 0.0f64));
    let ready = RwSignal::new(false);
    let layer = RwSignal::new(None::<OverlayId>);
    let timer = StoredValue::new_local(None::<i32>);
    let described = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let scroll_listener = StoredValue::new_local(None::<GlobalListener>);
    let press_listener = StoredValue::new_local(None::<GlobalListener>);
    let blur_listener = StoredValue::new_local(None::<GlobalListener>);
    let delay = if delay == 0 { 400 } else { delay };

    // 提示不是模态，也不能让 Esc 被别的浮层抢走：它自己占一层栈。
    // 这样「浮层开着、鼠标扫过按钮」时，第一次 Esc 先收提示是符合直觉的。
    let hide = move || {
        if let Some(id) = timer.get_value() {
            if let Some(w) = web_sys::window() {
                w.clear_timeout_with_handle(id);
            }
            timer.set_value(None);
        }
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
        if let Some(el) = described.get_value() {
            let _ = el.remove_attribute("aria-describedby");
            described.set_value(None);
        }
        shown.set(false);
        ready.set(false);
    };

    let show = move || {
        if layer.get_untracked().is_some() {
            return;
        }
        let Some(wrap) = wrap_ref.get() else { return };
        let id = host.push(OverlayKind::Tooltip, Callback::new(move |_| hide()));
        layer.set(Some(id));
        // 关联关系要挂在真正能聚焦的那个元素上：包装器只是用来测量的 span。
        let target = first_focusable(&wrap).unwrap_or_else(|| wrap.clone().into());
        let _ = target.set_attribute("aria-describedby", &tip_id_key.get_value());
        described.set_value(Some(target));

        // 先量尺寸再定位：提示的宽度由内容决定，不量就只能瞎猜。
        schedule(move || {
            let Some(tip) = tip_ref.get() else { return };
            let size = (tip.offset_width() as f64, tip.offset_height() as f64);
            let r = wrap.get_bounding_client_rect();
            let (vw, vh) = viewport();
            let (tw, th) = size;
            let (left, mut top) = match placement {
                TooltipPlacement::Bottom => (
                    r.left() + r.width() / 2.0 - tw / 2.0,
                    r.bottom() + GAP,
                ),
                TooltipPlacement::Top => (r.left() + r.width() / 2.0 - tw / 2.0, r.top() - th - GAP),
                TooltipPlacement::Left => (r.left() - tw - GAP, r.top() + r.height() / 2.0 - th / 2.0),
                TooltipPlacement::Right => (r.right() + GAP, r.top() + r.height() / 2.0 - th / 2.0),
            };
            // 底部越界翻上（默认方向），顶部越界翻下；横向永远夹取。
            match placement {
                TooltipPlacement::Bottom if top + th > vh - EDGE => top = r.top() - th - GAP,
                TooltipPlacement::Top if top < EDGE => top = r.bottom() + GAP,
                _ => {}
            }
            let max_left = (vw - tw - EDGE).max(EDGE);
            let max_top = (vh - th - EDGE).max(EDGE);
            pos.set((left.clamp(EDGE, max_left), top.clamp(EDGE, max_top)));
            ready.set(true);
            // 下一帧再打开过渡：同一帧里改 left/top 与 opacity 时，
            // 浏览器会把位移一起算进 transition，看起来像从旧位置飘过来。
            schedule(move || {
                if layer.get_untracked().is_some() {
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

    // 滚动 / 失焦 / 任意点击都收起：提示不能卡在屏幕中间。
    // scroll 用 capture —— 它不冒泡，内部滚动容器不会传到 window。
    if let Some(l) = listen_window("scroll", true, move |_| hide()) {
        scroll_listener.set_value(Some(l));
    }
    if let Some(l) = listen_document("pointerdown", move |_| hide()) {
        press_listener.set_value(Some(l));
    }
    if let Some(l) = listen_window("blur", false, move |_| hide()) {
        blur_listener.set_value(Some(l));
    }

    on_cleanup(move || {
        hide();
        scroll_listener.set_value(None);
        press_listener.set_value(None);
        blur_listener.set_value(None);
    });

    let tip_style = move || {
        if !ready.get() {
            return "left:0;top:0".to_string();
        }
        let (left, top) = pos.get();
        let z = layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Tooltip.base_z());
        format!("left:{left}px;top:{top}px;z-index:{z}")
    };

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
        <div
            class="c-tip"
            role="tooltip"
            id=tip_id_attr
            data-show=move || shown.get().then_some("1")
            node_ref=tip_ref
            style=tip_style
        >
            {label}
        </div>
    }
}
