//! 页内波浪条：连续滚动长页右侧的目录。
//!
//! 结构照抄 `css/src/components/09-section-wave.css`：
//! `.c-wave > .c-wave__track > .c-wave__slot > (.c-wave__bar + .c-wave__tip)`。
//! 每根条占多少纵向份额取决于当前项（高斯衰减），CSS 算不出来 ——
//! 只有量到轨道的真实高度后才知道，所以尺寸统一由这里的 `paint` 写内联样式。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

use crate::base::overlay::schedule;

/// 波形影响半径（以分区为单位）。
const SIGMA: f64 = 1.7;
/// 当前那根相对基线的最大放大。
const AMP: f64 = 1.5;
/// 横条最短 / 最长宽度（px）。
const BASE_W: f64 = 8.0;
const MAX_W: f64 = 30.0;
/// 每个分区预留的轨道高度（px）：条目少时整束居中，不拉满整条边。
const ROW_H: f64 = 34.0;
/// 判定「分区已进入视口」的顶线（px）。
const LINE: f64 = 140.0;
/// 点击跳转后多久内忽略滚动侦测（ms）—— 平滑滚动会途经中间分区。
const PICK_GUARD_MS: f64 = 700.0;

#[derive(Clone, PartialEq)]
pub struct WaveItem {
    pub id: String,
    pub label: String,
}

#[component]
pub fn SectionWave(
    items: Signal<Vec<WaveItem>>,
    #[prop(into, optional)] current: MaybeProp<usize>,
    #[prop(into, optional)] on_pick: Option<Callback<(usize, String)>>,
) -> impl IntoView {
    let root_ref = NodeRef::<html::Div>::new();
    let track_ref = NodeRef::<html::Div>::new();
    let internal = RwSignal::new(0usize);
    // 点击时间戳：用 performance.now() 的时间窗压住滚动侦测的抢权。
    // Leptos 0.8 的视图闭包要求 Send + Sync，`Rc<Cell<f64>>` 过不了这一关；
    // `StoredValue<_, LocalStorage>` 是同一块本地可写单元，句柄是 Copy + Sync。
    let last_pick = StoredValue::new_local(0.0f64);
    // 全局监听器句柄：析构即解绑，放进 StoredValue 就能活到组件卸载。
    let scroll_listener = StoredValue::new_local(None::<PassiveListener>);
    let resize_listener = StoredValue::new_local(None::<PassiveListener>);

    // 当前项：外部传了 `current` 就完全以它为真；否则用内部侦测值。
    let current_untracked = move || {
        current
            .get_untracked()
            .unwrap_or_else(|| internal.get_untracked())
    };

    let paint = move || {
        let (Some(track), Some(_)) = (track_ref.get_untracked(), root_ref.get_untracked()) else {
            return;
        };
        let n = items.get_untracked().len();
        if n == 0 {
            return;
        }
        // 条目少时别把整条边拉满：轨道最高只留 n * 34px，多出的交给外层居中。
        //
        // 显式走 HtmlElement 的固有方法：leptos prelude 的 `ElementExt::style`
        // 同名同参，靠接收者类型分不开。
        let track_style = web_sys::HtmlElement::style(&track);
        let _ = track_style.set_property("max-height", &format!("{}px", n as f64 * ROW_H));
        let height = track.client_height() as f64;
        if height <= 0.0 {
            return;
        }
        let active = current_untracked().min(n - 1);

        // 高斯权重 → 每个分区占的纵向份额。总高不变，放大的份额从别人那里挤出来。
        let mut sum = 0.0;
        let mut mags = Vec::with_capacity(n);
        for i in 0..n {
            let d = i as f64 - active as f64;
            let mag = 1.0 + AMP * (-(d * d) / (2.0 * SIGMA * SIGMA)).exp();
            sum += mag;
            mags.push(mag);
        }
        let unit = height / sum;

        // slot 是 `<div>`，但 HtmlCollection 只给 Element：统一当 HtmlElement 用。
        let slots = track.children();
        for (i, mag) in mags.iter().enumerate() {
            let Some(slot) = slots.item(i as u32) else {
                continue;
            };
            let Ok(slot) = slot.dyn_into::<web_sys::HtmlElement>() else {
                continue;
            };
            let _ = web_sys::HtmlElement::style(&slot)
                .set_property("height", &format!("{:.2}px", unit * mag));
            let _ = slot.set_attribute("data-current", if i == active { "true" } else { "false" });
            // 线宽走变量：真正那条线画在 `.c-wave__bar::before` 上。
            if let Ok(Some(bar)) = slot.query_selector(".c-wave__bar") {
                if let Some(bar) = bar.dyn_ref::<web_sys::HtmlElement>() {
                    let t = (mag - 1.0) / AMP;
                    let width = BASE_W + (MAX_W - BASE_W) * t;
                    let _ = web_sys::HtmlElement::style(bar)
                        .set_property("--bar-w", &format!("{width:.1}px"));
                }
            }
        }
    };

    // items / current 变了要重排；等一帧再量 —— Leptos 这轮渲染还没落定时
    // clientHeight 还是 0（`schedule` 就是 requestAnimationFrame）。
    Effect::new(move |_| {
        let _ = items.get();
        let _ = current.get();
        let _ = internal.get();
        let _ = root_ref.get();
        let _ = track_ref.get();
        schedule(paint);
    });

    // 滚动侦测只在非受控模式挂：外部有 `current` 时它是唯一真相。
    Effect::new(move |_| {
        // Effect 重跑（受控 ↔ 非受控、轨道重挂）先摘掉上一根监听器。
        scroll_listener.set_value(None);
        if current.get().is_some() {
            return;
        }
        let Some(root) = root_ref.get() else {
            return;
        };
        // 真正的滚动容器是 `.l-stage`。它**不一定**是祖先 —— 外壳里
        // `.c-wave` 与 `.l-stage` 是兄弟（波浪条要浮在滚动区旁边不跟着滚），
        // 所以 `closest` 找不到时还得看父节点下面有没有，最后才退回 window。
        let stage = root
            .closest(".l-stage")
            .ok()
            .flatten()
            .or_else(|| {
                root.parent_element()
                    .and_then(|p| p.query_selector(".l-stage").ok().flatten())
            });
        let target: web_sys::EventTarget = match stage {
            Some(stage) => stage.unchecked_into(),
            None => {
                let Some(window) = web_sys::window() else {
                    return;
                };
                window.unchecked_into()
            }
        };
        let listener = listen_passive(target, "scroll", move |_| {
            // 点击跳转的平滑滚动会途经中间分区，这段时间别抢当前项。
            if now_ms() - last_pick.get_value() < PICK_GUARD_MS {
                return;
            }
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                return;
            };
            let items_now = items.get_untracked();
            // 默认第一段：回到顶部时首段顶边可能还压在判定线下面
            // （外壳 20px 内边距 + 52px 顶栏 = 72，再加标题自身高度就过 140），
            // 那时一个都不越线，early-return 会让当前项永远停在上一次的位置。
            let mut found = Some(0usize);
            // 分区顶线越过 140px 就算进入视口；取最后一个越线的。
            for (i, item) in items_now.iter().enumerate() {
                if let Some(el) = document.get_element_by_id(&format!("sec-{}", item.id)) {
                    if el.get_bounding_client_rect().top() < LINE {
                        found = Some(i);
                    }
                }
            }
            let Some(i) = found else {
                return;
            };
            if internal.get_untracked() != i {
                internal.set(i);
            }
        });
        scroll_listener.set_value(Some(listener));
    });

    // 窗口尺寸变了，轨道高度跟着变，重排一次（过渡由 CSS 的 240ms 负责）。
    if let Some(window) = web_sys::window() {
        resize_listener.set_value(Some(listen_passive(
            window.unchecked_into(),
            "resize",
            move |_| schedule(paint),
        )));
    }
    // 组件卸载：StoredValue 里的监听器 Drop 时解绑。
    on_cleanup(move || {
        resize_listener.set_value(None);
        scroll_listener.set_value(None);
    });

    view! {
        // 空目录不渲染：留一条空轨既没用又挡住右侧的点击。
        <Show when=move || !items.get().is_empty() fallback=|| ()>
            <div class="c-wave" node_ref=root_ref>
                <div class="c-wave__track" node_ref=track_ref>
                    {move || {
                        items
                            .get()
                            .into_iter()
                            .enumerate()
                            .map(|(i, item)| {
                                let id = item.id.clone();
                                view! {
                                    <div class="c-wave__slot">
                                        <button
                                            class="c-wave__bar"
                                            type="button"
                                            aria-label=item.label.clone()
                                            on:click=move |ev: web_sys::MouseEvent| {
                                                ev.stop_propagation();
                                                last_pick.set_value(now_ms());
                                                if current.get_untracked().is_none() {
                                                    internal.set(i);
                                                }
                                                if let Some(cb) = on_pick {
                                                    cb.run((i, id.clone()));
                                                }
                                                scroll_to_section(&id);
                                            }
                                        ></button>
                                        <span class="c-wave__tip">{item.label}</span>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </div>
        </Show>
    }
}

/// 当前时间（ms）。`performance.now()` 单调递增，适合做时间窗判断。
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

/// 平滑滚到 `#sec-<id>` 顶部。`block: start` 与侦测用的 140px 顶线配合。
fn scroll_to_section(id: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(target) = document.get_element_by_id(&format!("sec-{id}")) else {
        return;
    };
    let options = web_sys::ScrollIntoViewOptions::new();
    options.set_behavior(web_sys::ScrollBehavior::Smooth);
    options.set_block(web_sys::ScrollLogicalPosition::Start);
    if let Some(target) = target.dyn_ref::<web_sys::HtmlElement>() {
        target.scroll_into_view_with_scroll_into_view_options(&options);
    }
}

/// passive 监听的句柄：Drop 时解绑。滚动 / 尺寸回调只读，passive 让浏览器
/// 不必等回调就能继续滚。放进 `StoredValue`（LocalStorage）即可跨线程边界存活。
struct PassiveListener {
    target: web_sys::EventTarget,
    event: &'static str,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for PassiveListener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, self.closure.as_ref().unchecked_ref());
    }
}

fn listen_passive(
    target: web_sys::EventTarget,
    event: &'static str,
    handler: impl FnMut(web_sys::Event) + 'static,
) -> PassiveListener {
    let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(web_sys::Event)>);
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(true);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
        event,
        closure.as_ref().unchecked_ref(),
        &options,
    );
    PassiveListener {
        target,
        event,
        closure,
    }
}
