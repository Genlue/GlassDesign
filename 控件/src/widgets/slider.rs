use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn Slider(
    #[prop(into, optional)] value: MaybeProp<f64>,
    #[prop(optional)] min: f64,
    #[prop(optional)] max: f64,
    #[prop(optional)] step: f64,
    #[prop(into, optional)] label: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_input: Option<Callback<f64>>,
    #[prop(optional)] on_change: Option<Callback<f64>>,
) -> impl IntoView {
    let max = if max <= min { min + 100.0 } else { max };
    let step = if step <= 0.0 { 1.0 } else { step };
    let track_ref = NodeRef::<html::Div>::new();
    let dragging = RwSignal::new(false);

    let raw = move || value.get().unwrap_or(min).clamp(min, max);
    let pos = move || ((raw() - min) / (max - min)).clamp(0.0, 1.0);
    let quantize = move |next: f64| {
        let steps = ((next - min) / step).round();
        (min + steps * step).clamp(min, max)
    };
    let emit = move |next: f64| {
        let q = quantize(next);
        if (q - raw()).abs() > f64::EPSILON {
            if let Some(cb) = on_input {
                cb.run(q);
            }
        }
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
    let from_x = move |x: f64| {
        let Some(el) = track_ref.get() else { return };
        let r = el.get_bounding_client_rect();
        if r.width() <= 0.0 {
            return;
        }
        let p = ((x - r.left()) / r.width()).clamp(0.0, 1.0);
        emit(min + p * (max - min));
    };

    view! {
        <div
            class="c-slider"
            role="slider"
            tabindex=if disabled { "-1" } else { "0" }
            aria-label=label
            aria-valuemin=min.to_string()
            aria-valuemax=max.to_string()
            aria-valuenow=move || raw().to_string()
            aria-disabled=disabled.then_some("true")
            data-dragging=move || dragging.get().then_some("1")
            style=move || {
                let p = pos();
                let rf = (p * 12.0).min((1.0 - p) * 12.0).min(1.0);
                let s = if dragging.get() { 1.6 } else { 1.0 };
                format!("--p:{p};--rf:{rf};--thumb-s:{s}")
            }
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                if disabled {
                    return;
                }
                ev.prevent_default();
                if let Some(target) = ev.current_target() {
                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                }
                dragging.set(true);
                from_x(ev.client_x() as f64);
            }
            on:pointermove=move |ev: web_sys::PointerEvent| {
                if dragging.get() {
                    from_x(ev.client_x() as f64);
                }
            }
            on:pointerup=move |_| {
                if !dragging.get() {
                    return;
                }
                dragging.set(false);
                if let Some(cb) = on_change { cb.run(raw()); }
            }
            on:pointercancel=move |_| { dragging.set(false); }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if disabled {
                    return;
                }
                let next = match ev.key().as_str() {
                    "ArrowLeft" | "ArrowDown" => raw() - step,
                    "ArrowRight" | "ArrowUp" => raw() + step,
                    "PageDown" => raw() - (max - min) / 10.0,
                    "PageUp" => raw() + (max - min) / 10.0,
                    "Home" => min,
                    "End" => max,
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
