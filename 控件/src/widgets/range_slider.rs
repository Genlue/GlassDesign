use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 双拇指区间滑块。
///
/// 和 `Slider` 的三处区别：
/// 1. **两颗拇指各自是独立的焦点目标**（`role="slider"`），容器只是 `role="group"`；
///    读屏要能分别念出「最小值 / 最大值」，把它们合并成一个控件就做不到。
/// 2. **指针落下时谁近谁动**，拖动过程中拇指身份不再互换 —— 否则拖左端越过后
///    手指还按在左边，动的却是右边那颗，视觉上像控件在跳。
/// 3. 位置与取值分开：取值量化到 `step`，位置跟着指针连续走（`--p1/--p2`），
///    所以低 step（如 50）也不会一卡一卡地跳格。
///
/// 布局不用测量：CSS 里 rail 两端各让半个拇指宽，`--p` 是 0..1 的比例，
/// 容器多宽都不会越界，也不需要 `ResizeObserver`。
#[component]
pub fn RangeSlider(
    #[prop(into, optional)] value: MaybeProp<(f64, f64)>,
    #[prop(optional)] min: f64,
    #[prop(optional)] max: f64,
    #[prop(optional)] step: f64,
    #[prop(into, optional)] label: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_input: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_change: Option<Callback<(f64, f64)>>,
) -> impl IntoView {
    let max = if max <= min { min + 100.0 } else { max };
    let step = if step <= 0.0 { 1.0 } else { step };
    let span = max - min;
    let rail_ref = NodeRef::<html::Div>::new();
    let thumb_refs = [NodeRef::<html::Div>::new(), NodeRef::<html::Div>::new()];
    // 拖动中的拇指：(side, 连续位置)。离开拖动就清空，位置重新由取值推出。
    let drag = RwSignal::new(None::<(usize, f64)>);
    let moved = RwSignal::new(false);

    // 量化从 min 起算；末尾抹掉 1e-6 以下的浮点尾巴，aria-valuenow 才不会出现
    // 0.30000000000000004 这种读数。
    let quantize = move |v: f64| {
        let q = min + ((v - min) / step).round() * step;
        (q.clamp(min, max) * 1_000_000.0).round() / 1_000_000.0
    };
    // 乱序 / 越界 / NaN 一律归一，调用方不用保证 value 已排序。
    let pair = move || {
        let (a, b) = value.get().unwrap_or((min, max));
        let (a, b) = if a.is_finite() && b.is_finite() {
            (quantize(a), quantize(b))
        } else {
            (min, max)
        };
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    };
    let pct = move |v: f64| ((v - min) / span).clamp(0.0, 1.0);
    let pos_of = move |side: usize| -> f64 {
        if let Some((s, p)) = drag.get() {
            if s == side {
                return p;
            }
        }
        let (a, b) = pair();
        pct(if side == 0 { a } else { b })
    };
    let dragging = move |side: usize| drag.get().map(|(s, _)| s == side).unwrap_or(false);
    let focus_thumb = move |side: usize| {
        if let Some(el) = thumb_refs[side].get() {
            let _ = el.focus();
        }
    };

    let emit = move |next: (f64, f64), commit: bool| {
        if next == pair() {
            return;
        }
        if let Some(cb) = on_input {
            cb.run(next);
        }
        if commit {
            if let Some(cb) = on_change {
                cb.run(next);
            }
        }
    };

    // 把某一侧推到连续位置 p：位置先夹在另一侧之内（拖动中不互换身份），
    // 取值再量化。两者分开写入，拇指才不会在低 step 下跳格。
    let apply = move |side: usize, p: f64| {
        let (a, b) = pair();
        let (pa, pb) = (pct(a), pct(b));
        let (pe, next) = if side == 0 {
            let pe = p.clamp(0.0, 1.0).min(pb);
            (pe, (quantize(min + pe * span).min(b), b))
        } else {
            let pe = p.clamp(0.0, 1.0).max(pa);
            (pe, (a, quantize(min + pe * span).max(a)))
        };
        drag.set(Some((side, pe)));
        if next != (a, b) {
            moved.set(true);
        }
        emit(next, false);
    };

    let from_x = move |x: f64| -> Option<f64> {
        let el = rail_ref.get()?;
        let r = el.get_bounding_client_rect();
        if r.width() <= 0.0 {
            return None;
        }
        Some(((x - r.left()) / r.width()).clamp(0.0, 1.0))
    };

    let on_key = move |ev: web_sys::KeyboardEvent, side: usize| {
        if disabled {
            return;
        }
        let (a, b) = pair();
        let cur = if side == 0 { a } else { b };
        let big = span / 10.0;
        let target = match ev.key().as_str() {
            "ArrowLeft" | "ArrowDown" => cur - step,
            "ArrowRight" | "ArrowUp" => cur + step,
            "PageDown" => cur - big,
            "PageUp" => cur + big,
            "Home" => min,
            "End" => max,
            _ => return,
        };
        ev.prevent_default();
        let q = quantize(target);
        // 到头就停住，不把另一颗顶走：键盘用户按下 ← 期待的是「这一端变小」。
        let next = if side == 0 {
            (q.min(b), b)
        } else {
            (a, q.max(a))
        };
        emit(next, true);
    };

    let group = if label.is_empty() {
        "区间滑块".to_string()
    } else {
        label.clone()
    };

    let thumb = {
        let group = group.clone();
        move |side: usize| -> AnyView {
            let name = if side == 0 {
                format!("{group}最小值")
            } else {
                format!("{group}最大值")
            };
            view! {
                <div
                    class="c-rangeslider__thumb"
                    role="slider"
                    tabindex=if disabled { "-1" } else { "0" }
                    aria-label=name
                    aria-orientation="horizontal"
                    aria-valuemin=min.to_string()
                    aria-valuemax=max.to_string()
                    aria-valuenow=move || {
                        let (a, b) = pair();
                        (if side == 0 { a } else { b }).to_string()
                    }
                    aria-disabled=disabled.then_some("true")
                    data-side=if side == 0 { "lo" } else { "hi" }
                    data-dragging=move || dragging(side).then_some("1")
                    // 折射强度跟端点到边缘的距离走：越靠边越少折射，和单值滑块同一套。
                    style=move || {
                        let p = pos_of(side);
                        let rf = (p * 12.0).min((1.0 - p) * 12.0).min(1.0);
                        let s = if dragging(side) { 1.06 } else { 1.0 };
                        format!("--p:{p};--rf:{rf};--thumb-s:{s}")
                    }
                    node_ref=thumb_refs[side]
                    on:keydown=move |ev| on_key(ev, side)
                >
                    <div class="c-rangeslider__mask"></div>
                </div>
            }
            .into_any()
        }
    };

    view! {
        <div
            class="c-rangeslider"
            role="group"
            aria-label=group
            aria-disabled=disabled.then_some("true")
            data-disabled=disabled.then_some("1")
            // CSS 读 --p-lo / --p-hi；--p1 / --p2 是同一对值的对外名字，
            // 两套都直接写实数，不靠 var() 间接传，免得某个引擎解析顺序不同。
            style=move || {
                let (p1, p2) = (pos_of(0), pos_of(1));
                format!("--p1:{p1:.6};--p2:{p2:.6};--p-lo:{p1:.6};--p-hi:{p2:.6}")
            }
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                if disabled {
                    return;
                }
                // 指针落在轨道任意处都要能开始拖，所以监听挂在容器上而不是拇指上。
                ev.prevent_default();
                let Some(p) = from_x(ev.client_x() as f64) else { return };
                let v = min + p * span;
                let (a, b) = pair();
                // 谁近谁动：落在两端外侧直接取该端，中间就近，等距取左。
                let side = if v <= a {
                    0
                } else if v >= b {
                    1
                } else if v - a <= b - v {
                    0
                } else {
                    1
                };
                if let Some(target) = ev.current_target() {
                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                }
                moved.set(false);
                drag.set(Some((side, p)));
                apply(side, p);
                // 焦点跟着「现在被拖的那颗」走，松开后键盘接手不会落在另一端。
                focus_thumb(side);
            }
            on:pointermove=move |ev: web_sys::PointerEvent| {
                if let Some((side, _)) = drag.get() {
                    if let Some(p) = from_x(ev.client_x() as f64) {
                        apply(side, p);
                    }
                }
            }
            on:pointerup=move |_| {
                if drag.get_untracked().is_none() {
                    return;
                }
                drag.set(None);
                // 只有真的动过才发 change —— 原地按一下不该惊动订阅方。
                if moved.get_untracked() {
                    if let Some(cb) = on_change {
                        cb.run(pair());
                    }
                }
                moved.set(false);
            }
            on:pointercancel=move |_| {
                drag.set(None);
                moved.set(false);
            }
        >
            <div class="c-rangeslider__rail" node_ref=rail_ref>
                <div class="c-rangeslider__track">
                    <div class="c-rangeslider__fill"></div>
                </div>
                {thumb(0)}
                {thumb(1)}
            </div>
        </div>
    }
}
