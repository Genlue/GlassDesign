use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn Slider(
    #[prop(into, optional)] value: MaybeProp<f64>,
    #[prop(into, optional)] min: MaybeProp<f64>,
    #[prop(into, optional)] max: MaybeProp<f64>,
    #[prop(into, optional)] step: MaybeProp<f64>,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] disabled: MaybeProp<bool>,
    #[prop(optional)] on_input: Option<Callback<f64>>,
    #[prop(optional)] on_change: Option<Callback<f64>>,
    #[prop(optional)] on_cancel: Option<Callback<()>>,
) -> impl IntoView {
    let slider_ref = NodeRef::<html::Div>::new();
    let track_ref = NodeRef::<html::Div>::new();
    let dragging = RwSignal::new(false);
    let drag_val = RwSignal::new(Option::<f64>::None);
    let active_pointer = StoredValue::new_local(Option::<i32>::None);

    let min_val = move || {
        let v = min.get().unwrap_or(0.0);
        if v.is_finite() { v } else { 0.0 }
    };
    let max_val = move || {
        let lo = min_val();
        let hi = max.get().unwrap_or(100.0);
        if !hi.is_finite() || hi <= lo {
            lo + 100.0
        } else {
            hi
        }
    };
    let step_val = move || {
        let s = step.get().unwrap_or(1.0);
        if !s.is_finite() || s <= 0.0 {
            1.0
        } else {
            s
        }
    };
    let is_disabled = move || disabled.get().unwrap_or(false);

    // 拖拽途中若控件变为禁用，立即终止拖拽并通知取消，避免外部预览态残留
    Effect::new(move |_| {
        if is_disabled() && dragging.get_untracked() {
            if let (Some(el), Some(pid)) = (slider_ref.get_untracked(), active_pointer.get_value()) {
                let _ = el.release_pointer_capture(pid);
            }
            active_pointer.set_value(None);
            dragging.set(false);
            drag_val.set(None);
            if let Some(cb) = on_cancel {
                cb.run(());
            }
        }
    });

    let raw = move || {
        let lo = min_val();
        let hi = max_val();
        let v = drag_val.get().or_else(|| value.get()).unwrap_or(lo);
        if v.is_finite() {
            v.clamp(lo, hi)
        } else {
            lo
        }
    };
    let pos = move || {
        let lo = min_val();
        let hi = max_val();
        ((raw() - lo) / (hi - lo)).clamp(0.0, 1.0)
    };
    let quantize = move |next: f64| {
        let lo = min_val();
        let hi = max_val();
        let st = step_val();
        if !next.is_finite() {
            return lo;
        }
        let steps = ((next - lo) / st).round();
        (lo + steps * st).clamp(lo, hi)
    };
    let calc_from_x = move |x: f64| -> Option<f64> {
        let el = track_ref.get_untracked()?;
        let r = el.get_bounding_client_rect();
        if r.width() <= 0.0 {
            return None;
        }
        let lo = min_val();
        let hi = max_val();
        let p = ((x - r.left()) / r.width()).clamp(0.0, 1.0);
        Some(quantize(lo + p * (hi - lo)))
    };
    let commit = move |next: f64| {
        let q = quantize(next);
        if let Some(cb) = on_input {
            cb.run(q);
        }
        if let Some(cb) = on_change {
            cb.run(q);
        }
    };
    let cancel_drag = move || {
        if !dragging.get_untracked() {
            return;
        }
        if let (Some(el), Some(pid)) = (slider_ref.get_untracked(), active_pointer.get_value()) {
            let _ = el.release_pointer_capture(pid);
        }
        active_pointer.set_value(None);
        dragging.set(false);
        drag_val.set(None);
        if let Some(cb) = on_cancel {
            cb.run(());
        }
    };

    view! {
        <div
            class="c-slider"
            node_ref=slider_ref
            role="slider"
            tabindex=move || if is_disabled() { "-1" } else { "0" }
            aria-label=label
            aria-valuemin=move || min_val().to_string()
            aria-valuemax=move || max_val().to_string()
            aria-valuenow=move || raw().to_string()
            aria-disabled=move || is_disabled().then_some("true")
            data-dragging=move || dragging.get().then_some("1")
            style=move || {
                let p = pos();
                let rf = (p * 12.0).min((1.0 - p) * 12.0).min(1.0);
                let s = if dragging.get() { 1.6 } else { 1.0 };
                format!("--p:{p};--rf:{rf};--thumb-s:{s}")
            }
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                if ev.button() != 0 || is_disabled() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                let pid = ev.pointer_id();
                if let Some(el) = slider_ref.get_untracked() {
                    let _ = el.set_pointer_capture(pid);
                } else if let Some(target) = ev.current_target() {
                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                        let _ = el.set_pointer_capture(pid);
                    }
                }
                active_pointer.set_value(Some(pid));
                dragging.set(true);
                if let Some(q) = calc_from_x(ev.client_x() as f64) {
                    drag_val.set(Some(q));
                    if let Some(cb) = on_input {
                        cb.run(q);
                    }
                }
            }
            on:pointermove=move |ev: web_sys::PointerEvent| {
                if !dragging.get_untracked() || active_pointer.get_value() != Some(ev.pointer_id()) {
                    return;
                }
                if let Some(q) = calc_from_x(ev.client_x() as f64) {
                    let changed = drag_val
                        .get_untracked()
                        .map_or(true, |prev| (q - prev).abs() > f64::EPSILON);
                    if changed {
                        drag_val.set(Some(q));
                        if let Some(cb) = on_input {
                            cb.run(q);
                        }
                    }
                }
            }
            on:pointerup=move |ev: web_sys::PointerEvent| {
                if !dragging.get_untracked() || active_pointer.get_value() != Some(ev.pointer_id()) {
                    return;
                }
                let pid = ev.pointer_id();
                if let Some(el) = slider_ref.get_untracked() {
                    let _ = el.release_pointer_capture(pid);
                }
                let final_v = calc_from_x(ev.client_x() as f64)
                    .or_else(|| drag_val.get_untracked())
                    .unwrap_or_else(raw);
                active_pointer.set_value(None);
                dragging.set(false);
                drag_val.set(None);
                if let Some(cb) = on_change {
                    cb.run(final_v);
                }
            }
            on:pointercancel=move |ev: web_sys::PointerEvent| {
                if active_pointer.get_value() == Some(ev.pointer_id()) {
                    cancel_drag();
                }
            }
            on:lostpointercapture=move |ev: web_sys::PointerEvent| {
                if dragging.get_untracked() && active_pointer.get_value() == Some(ev.pointer_id()) {
                    cancel_drag();
                }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if is_disabled() {
                    return;
                }
                let lo = min_val();
                let hi = max_val();
                let st = step_val();
                let next = match ev.key().as_str() {
                    "ArrowLeft" | "ArrowDown" => raw() - st,
                    "ArrowRight" | "ArrowUp" => raw() + st,
                    "PageDown" => raw() - (hi - lo) / 10.0,
                    "PageUp" => raw() + (hi - lo) / 10.0,
                    "Home" => lo,
                    "End" => hi,
                    _ => return,
                };
                ev.prevent_default();
                commit(next);
            }
        >
            <div class="c-slider__track" node_ref=track_ref>
                <div class="c-slider__fill"></div>
            </div>
            <div class="c-slider__thumb">
                <div class="c-slider__mask"></div>
            </div>
        </div>
    }
}
