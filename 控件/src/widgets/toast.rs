use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::base::overlay::Z_TOAST;
use crate::widgets::inline_message::Tone;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ToastItem {
    pub id: u32,
    pub message: String,
    pub tone: Tone,
    pub icon: Option<IconName>,
}

/// 退场动画时长（ms）。必须与 `14-toast.css` 里
/// `.c-toast-slot[data-leaving="1"] .c-toast` 的 animation-duration 相等：
/// CSS 还没播完就把节点摘掉，看到的是「淡到一半突然消失」。
const EXIT_MS: i32 = 180;

/// 入场后停留多久（ms）。
const STAY_MS: i32 = 3200;

/// 挤兑动画时长（ms）。比入场稍长：这是「别人的位置被改」，慢一点才跟得上。
const FLIP_MS: i32 = 340;

thread_local! {
    static TOASTS: RwSignal<Vec<ToastItem>> = RwSignal::new(Vec::new());
    static NEXT_ID: Cell<u32> = const { Cell::new(0) };
    /// 上一次布局后，每条提示在视口里的顶边坐标 —— FLIP 的「前一半」。
    /// 表里没有的 id 就是刚挂上来的（走 CSS 入场），不需要位移补偿。
    static TOPS: RefCell<HashMap<u32, f64>> = RefCell::new(HashMap::new());
    /// 一次列表变化只排一次测量。连发三条提示时不会叠三次 rAF。
    static FLIP_QUEUED: Cell<bool> = const { Cell::new(false) };
}

fn icon_for(tone: Tone) -> IconName {
    match tone {
        Tone::Info => IconName::INFO,
        Tone::Success => IconName::CIRCLE_CHECK,
        Tone::Warning => IconName::TRIANGLE_ALERT,
        Tone::Danger => IconName::X_CIRCLE,
    }
}

/// 发一条提示。3.2 秒后自动移除。
pub fn toast(message: impl Into<String>) -> u32 {
    toast_with(message, Tone::Info, None)
}

pub fn toast_with(message: impl Into<String>, tone: Tone, icon: Option<IconName>) -> u32 {
    let id = NEXT_ID.with(|n| {
        let v = n.get() + 1;
        n.set(v);
        v
    });
    let item = ToastItem {
        id,
        message: message.into(),
        tone,
        icon: icon.or_else(|| Some(icon_for(tone))),
    };
    let signal = TOASTS.with(|s| *s);
    signal.update(|list| list.push(item));
    schedule_remove(signal, id);
    id
}

/// 定时器链：停留 → 挂 `data-leaving`（CSS 播退场）→ 到点出列（触发 FLIP 合拢）。
fn schedule_remove(signal: RwSignal<Vec<ToastItem>>, id: u32) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let start_exit = wasm_bindgen::closure::Closure::once_into_js(move || {
        if let Some(el) = slot_of(id) {
            let _ = el.set_attribute("data-leaving", "1");
        }
        let Some(w) = web_sys::window() else { return };
        let drop_it = wasm_bindgen::closure::Closure::once_into_js(move || {
            // 出列会触发 ToastHost 里的 FLIP：下面的提示「合拢」而不是瞬移。
            signal.update(|list| list.retain(|t| t.id != id));
        });
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
            drop_it.unchecked_ref::<js_sys::Function>(),
            EXIT_MS,
        );
    });

    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        start_exit.unchecked_ref::<js_sys::Function>(),
        STAY_MS,
    );
}

fn slot_of(id: u32) -> Option<web_sys::Element> {
    let doc = web_sys::window()?.document()?;
    doc.query_selector(&format!(".c-toast-slot[data-id=\"{id}\"]"))
        .ok()
        .flatten()
}

fn slots() -> Vec<web_sys::Element> {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return Vec::new();
    };
    let Ok(list) = doc.query_selector_all(".c-toast-host .c-toast-slot") else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.get(i))
        .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
        .collect()
}

fn slot_id(el: &web_sys::Element) -> Option<u32> {
    el.get_attribute("data-id").and_then(|s| s.parse().ok())
}

/// 列表变了以后补一次 FLIP。
///
/// 为什么量的是「当前」位置：`TOPS` 里存的是**上一次变化之后**的坐标，
/// 也就是这次的「前」。所以不需要在改动 DOM 之前抢着快照 —— 少一个时序依赖。
///
/// 反向位移和放开必须在同一帧里做完：中间夹一次 `offsetWidth` 强制回流，
/// 浏览器才会把「先瞬移过去」当成起点，否则会连着起点一起过渡，看起来没动。
fn queue_flip() {
    if FLIP_QUEUED.with(|c| c.replace(true)) {
        return;
    }
    let Some(window) = web_sys::window() else {
        FLIP_QUEUED.with(|c| c.set(false));
        return;
    };
    let cb = wasm_bindgen::closure::Closure::once_into_js(|| {
        FLIP_QUEUED.with(|c| c.set(false));
        run_flip();
    });
    let _ = window.request_animation_frame(cb.unchecked_ref::<js_sys::Function>());
}

fn run_flip() {
    let list = slots();
    if list.is_empty() {
        TOPS.with(|m| m.borrow_mut().clear());
        return;
    }

    let mut next: HashMap<u32, f64> = HashMap::with_capacity(list.len());
    let mut moved: Vec<(web_sys::Element, f64)> = Vec::new();

    for el in &list {
        let Some(id) = slot_id(el) else { continue };
        let top = el.get_bounding_client_rect().top();
        next.insert(id, top);
        if let Some(prev) = TOPS.with(|m| m.borrow().get(&id).copied()) {
            let dy = prev - top;
            if dy.abs() > 0.5 {
                moved.push((el.clone(), dy));
            }
        }
    }

    TOPS.with(|m| *m.borrow_mut() = next);

    for (el, dy) in &moved {
        if let Some(html) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = html.style().set_property("transition", "none");
            let _ = html
                .style()
                .set_property("transform", &format!("translateY({dy:.2}px)"));
        }
        // 强制回流：让上面这帧「瞬移」成为过渡起点
        let _ = el.get_bounding_client_rect();
    }

    if moved.is_empty() {
        return;
    }

    let els: Vec<web_sys::Element> = moved.into_iter().map(|(e, _)| e).collect();
    for el in &els {
        if let Some(html) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = html.style().set_property(
                "transition",
                &format!("transform {FLIP_MS}ms cubic-bezier(0.22, 0.9, 0.24, 1)"),
            );
            let _ = html.style().set_property("transform", "translateY(0)");
        }
    }

    // 过渡完把行内样式清掉：留着一个 `translateY(0)` 会让槽变成新的包含块，
    // 玻璃的 backdrop 采样范围跟着它走，边缘会发虚。
    let Some(window) = web_sys::window() else { return };
    let cleanup = wasm_bindgen::closure::Closure::once_into_js(move || {
        for el in els {
            let _ = el.remove_attribute("style");
        }
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cleanup.unchecked_ref::<js_sys::Function>(),
        FLIP_MS + 40,
    );
}

#[component]
pub fn ToastHost() -> impl IntoView {
    let signal = TOASTS.with(|s| *s);

    // 订阅列表：每次增删排一次 FLIP。effect 跑在 Leptos 打完 DOM 补丁之后，
    // 但为稳妥仍然把测量放进 rAF —— 那一帧一定晚于 DOM 改动、早于绘制。
    Effect::new(move |_| {
        signal.get();
        queue_flip();
    });

    view! {
        // 层级来自宿主常量表：提示条不参与 Esc 栈，但必须压在模态遮罩之上。
        <div class="c-toast-host" role="status" aria-live="polite" data-layer-ignore=""
            style=format!("z-index:{Z_TOAST}")>
            <For each=move || signal.get() key=|t| t.id let:item>
                <div class="c-toast-slot" data-id=item.id.to_string()>
                    <div class=format!("c-toast c-toast--{}", match item.tone {
                        Tone::Info => "info",
                        Tone::Success => "success",
                        Tone::Warning => "warning",
                        Tone::Danger => "danger",
                    })>
                        {item.icon.map(|name| view! { <Icon name=name size=14 /> })}
                        {item.message}
                    </div>
                </div>
            </For>
        </div>
    }
}
