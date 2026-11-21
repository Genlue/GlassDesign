//! Standalone pixel divider for application-owned layouts.
use leptos::{html, prelude::*};

#[derive(Clone, Copy)]
struct Drag {
    pointer: i32,
    x: i32,
    value: u32,
}

#[component]
pub fn SplitGrip(
    value: Signal<u32>,
    min: Signal<u32>,
    max: Signal<u32>,
    #[prop(into)] label: String,
    #[prop(into, optional)] controls: String,
    on_change: Callback<u32>,
) -> impl IntoView {
    let grip = NodeRef::<html::Div>::new();
    let drag = RwSignal::new(None::<Drag>);
    let bounds = move || {
        let low = min.get();
        (low, max.get().max(low))
    };
    let current = move || {
        let (low, high) = bounds();
        value.get().clamp(low, high)
    };
    let change = move |next: i64| {
        let (low, high) = bounds();
        let next = next.clamp(i64::from(low), i64::from(high)) as u32;
        if next != current() {
            on_change.run(next);
        }
    };
    let move_pointer = move |event: &web_sys::PointerEvent| {
        if let Some(start) = drag
            .get_untracked()
            .filter(|d| d.pointer == event.pointer_id())
        {
            let delta = i64::from(event.client_x()) - i64::from(start.x);
            change(i64::from(start.value) + delta);
        }
    };
    let end_pointer = move |event: web_sys::PointerEvent| {
        if drag
            .get_untracked()
            .is_some_and(|d| d.pointer == event.pointer_id())
        {
            drag.set(None);
        }
        // Capture is automatically released after pointerup / pointercancel.
    };

    view! {
        <div
            class="c-resizer" data-axis="x" data-no-drag="true" node_ref=grip
            role="separator" aria-orientation="vertical" tabindex="0"
            aria-label=label aria-controls=(!controls.is_empty()).then_some(controls)
            aria-valuemin=move || bounds().0.to_string()
            aria-valuemax=move || bounds().1.to_string()
            aria-valuenow=move || current().to_string()
            aria-valuetext=move || format!("{} px", current())
            data-active=move || drag.get().map(|_| "1")
            on:pointerdown=move |event: web_sys::PointerEvent| {
                if event.button() != 0 || !event.is_primary() || drag.get_untracked().is_some() {
                    return;
                }
                if let Some(element) = grip.get_untracked() {
                    if element.set_pointer_capture(event.pointer_id()).is_err() {
                        return;
                    }
                    event.prevent_default();
                    let _ = element.focus();
                    drag.set(Some(Drag { pointer: event.pointer_id(), x: event.client_x(), value: current() }));
                }
            }
            on:pointermove=move |event: web_sys::PointerEvent| move_pointer(&event)
            on:pointerup=move |event: web_sys::PointerEvent| {
                move_pointer(&event);
                end_pointer(event);
            }
            on:pointercancel=end_pointer
            on:lostpointercapture=end_pointer
            on:keydown=move |event: web_sys::KeyboardEvent| {
                if event.is_composing() || event.ctrl_key() || event.alt_key() || event.meta_key() {
                    return;
                }
                let step = if event.shift_key() { 24 } else { 8 };
                let next = match event.key().as_str() {
                    "ArrowLeft" => i64::from(current()) - step,
                    "ArrowRight" => i64::from(current()) + step,
                    "Home" => i64::from(bounds().0),
                    "End" => i64::from(bounds().1),
                    _ => return,
                };
                event.prevent_default();
                change(next);
            }
        ></div>
    }
}
