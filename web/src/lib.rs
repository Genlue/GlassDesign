//! Browser-only adapter for mounting real GlassDesign components from a JS host.

#![cfg(target_arch = "wasm32")]

mod props;
mod slots;
mod widget;

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use glass_design::OverlayHost;
use js_sys::{Function, WeakSet};
use leptos::mount::{mount_to, UnmountHandle};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use props::error;
use slots::Slots;
use widget::{LiveProps, Widget};

thread_local! {
    static HOSTS: WeakSet<HtmlElement> = WeakSet::new_typed();
    static OVERLAYS: RefCell<Weak<SharedOverlays>> = const { RefCell::new(Weak::new()) };
}

struct SharedOverlays {
    _owner: Owner,
    host: OverlayHost,
}

impl SharedOverlays {
    fn acquire() -> Rc<Self> {
        OVERLAYS.with_borrow_mut(|cached| {
            if let Some(active) = cached.upgrade() {
                return active;
            }
            let owner = Owner::new();
            let host = owner.with(OverlayHost::provide);
            let active = Rc::new(Self {
                _owner: owner,
                host,
            });
            *cached = Rc::downgrade(&active);
            active
        })
    }
}

struct Host(HtmlElement);

impl Host {
    fn reserve(element: HtmlElement) -> Result<Self, JsValue> {
        HOSTS.with(|hosts| {
            if hosts.has(&element) {
                return Err(error(
                    "This element already has a GlassDesign widget; dispose it first",
                ));
            }
            hosts.add(&element);
            Ok(Self(element))
        })
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        HOSTS.with(|hosts| hosts.delete(&self.0));
    }
}

struct Events {
    live: Cell<bool>,
    emit: Function,
}

#[derive(Clone)]
struct Emitter(SendWrapper<Rc<Events>>);

impl Emitter {
    fn queue(&self, value: Option<String>) {
        // Do not call JS from inside a Leptos event. The host is allowed to
        // replace this widget from emit(), including after a menu selection.
        let events = Rc::downgrade(&self.0);
        leptos::prelude::queue_microtask(move || {
            let Some(events) = events.upgrade().filter(|e| e.live.get()) else {
                return;
            };
            let result = if let Some(value) = value {
                events
                    .emit
                    .call1(&JsValue::UNDEFINED, &JsValue::from_str(&value))
            } else {
                events.emit.call0(&JsValue::UNDEFINED)
            };
            // Report host callback failures without unwinding through the renderer.
            if let Err(exception) = result {
                web_sys::console::error_1(&exception);
            }
        });
    }

    fn action(self) -> Callback<()> {
        Callback::new(move |_| self.queue(None))
    }

    fn change(self, value: RwSignal<String>, skip_equal: bool) -> Callback<String> {
        Callback::new(move |next: String| {
            if skip_equal && value.with_untracked(|current| *current == next) {
                return;
            }
            value.set(next.clone());
            self.queue(Some(next));
        })
    }

    fn search_input(self, value: RwSignal<String>) -> Callback<String> {
        // Some browsers send a final non-composing input after compositionend.
        self.change(value, true)
    }
}

#[wasm_bindgen]
pub struct WidgetHandle {
    mounted: Option<MountedWidget>,
}

struct MountedWidget {
    host: Host,
    render: Option<UnmountHandle<<AnyView as Render>::State>>,
    owner: Owner,
    slots: Slots,
    live_props: LiveProps,
    events: Rc<Events>,
    // Kept alive until after render is dropped (Select/Menu clean up their layers there).
    _overlays: Option<Rc<SharedOverlays>>,
}

impl Drop for MountedWidget {
    fn drop(&mut self) {
        self.events.live.set(false);
        self.slots.restore(&self.host.0);
        drop(self.render.take());
        // Portal effects can still hold child owners across an async turn. Clean
        // up the whole widget explicitly before releasing the shared overlay host.
        self.owner.cleanup();
    }
}

#[wasm_bindgen]
impl WidgetHandle {
    /// Patch controlled values without remounting or emitting a user event.
    pub fn update(&self, patch: JsValue) -> Result<(), JsValue> {
        let mounted = self
            .mounted
            .as_ref()
            .ok_or_else(|| error("Widget is disposed"))?;
        mounted.live_props.update(&patch)
    }

    /// Unmount the component and dispose of its effects and any open popover.
    /// Safe to call more than once, until the JS wrapper itself is freed.
    pub fn dispose(&mut self) {
        drop(self.mounted.take());
    }
}

/// Append a GlassDesign widget to a host in the current document's light DOM.
/// Actions emit zero arguments; controls emit one string. Events arrive in a microtask.
#[wasm_bindgen]
pub fn mount_widget(
    element: HtmlElement,
    kind: &str,
    props: JsValue,
    emit: Function,
) -> Result<WidgetHandle, JsValue> {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return Err(error("GlassDesign requires a browser document"));
    };
    if !element.is_instance_of::<HtmlElement>() {
        return Err(error("element must be an HTMLElement"));
    }
    if !document.is_same_node(element.owner_document().as_deref())
        || (element.is_connected() && !document.is_same_node(Some(&element.get_root_node())))
    {
        return Err(error(
            "GlassDesign requires a host in the current document's light DOM",
        ));
    }
    // wasm-bindgen's extern object arguments are not JS runtime type checks.
    if !emit.is_function() {
        return Err(error("emit must be a function"));
    }
    let host = Host::reserve(element)?;
    let widget = Widget::parse(kind, &props)?;
    if widget.uses_overlay() && host.0.closest("dialog")?.is_some() {
        return Err(error(
            "Use native dialog controls: this GlassDesign overlay is portaled to body",
        ));
    }
    let slots = Slots::collect(kind, &host.0)?;
    let has_right_slot = slots.has_end();
    let events = Rc::new(Events {
        live: Cell::new(true),
        emit,
    });
    let emitter = Emitter(SendWrapper::new(Rc::clone(&events)));
    let overlays = widget.uses_overlay().then(SharedOverlays::acquire);
    let overlay_host = overlays.as_ref().map(|o| o.host);
    let owner = Owner::new();
    let live_props = owner.with(|| widget.state());
    let render = owner.with(|| {
        mount_to(host.0.clone(), move || {
            if let Some(host) = overlay_host {
                provide_context(host);
            }
            widget.render(emitter, has_right_slot, live_props)
        })
    });
    let mut mounted = MountedWidget {
        host,
        render: Some(render),
        owner,
        slots,
        live_props,
        events,
        _overlays: overlays,
    };
    mounted.slots.project(&mounted.host.0)?;
    Ok(WidgetHandle {
        mounted: Some(mounted),
    })
}
