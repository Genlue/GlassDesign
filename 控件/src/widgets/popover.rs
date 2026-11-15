use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::overlay::{
    active_element, first_focusable, listen_document, schedule, GlobalListener, OverlayHost,
    OverlayId, OverlayKind,
};

/// 贴边安全距离：与菜单共用同一套几何约定（8px），窗口圆角不会切掉面板。
const EDGE: f64 = 8.0;

/// 气泡相对触发器的方位。`Start` 按左/上对齐，`End` 按右对齐。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum PopoverPlacement {
    #[default]
    BottomStart,
    BottomEnd,
    TopStart,
    TopEnd,
    RightStart,
    LeftStart,
}

/// 视口尺寸。`inner_width/inner_height` 在 web-sys 里是 `JsValue`，统一转一次。
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

/// 先按声明的方位算，再按越界自动翻转（下→上、右→左），最后夹进视口。
///
/// 三步必须按这个顺序：只夹取不翻转的话，靠近底部的触发器会把气泡挤到贴着
/// 触发器上沿的位置，看起来像「盖住了自己」；只翻转不夹取的话，窗口比气泡还
/// 窄时又会伸到屏幕外。
fn compute_pos(
    placement: PopoverPlacement,
    trigger: &web_sys::DomRect,
    panel: (f64, f64),
    offset: f64,
) -> (f64, f64) {
    let (pw, ph) = panel;
    let (vw, vh) = viewport();
    let (mut left, mut top) = match placement {
        PopoverPlacement::BottomStart => (trigger.left(), trigger.bottom() + offset),
        PopoverPlacement::BottomEnd => (trigger.right() - pw, trigger.bottom() + offset),
        PopoverPlacement::TopStart => (trigger.left(), trigger.top() - ph - offset),
        PopoverPlacement::TopEnd => (trigger.right() - pw, trigger.top() - ph - offset),
        PopoverPlacement::RightStart => (trigger.right() + offset, trigger.top()),
        PopoverPlacement::LeftStart => (trigger.left() - pw - offset, trigger.top()),
    };

    match placement {
        PopoverPlacement::BottomStart | PopoverPlacement::BottomEnd => {
            if top + ph > vh - EDGE {
                top = trigger.top() - ph - offset;
            }
        }
        PopoverPlacement::TopStart | PopoverPlacement::TopEnd => {
            if top < EDGE {
                top = trigger.bottom() + offset;
            }
        }
        PopoverPlacement::RightStart => {
            if left + pw > vw - EDGE {
                left = trigger.left() - pw - offset;
            }
        }
        PopoverPlacement::LeftStart => {
            if left < EDGE {
                left = trigger.right() + offset;
            }
        }
    }

    let max_left = (vw - pw - EDGE).max(EDGE);
    let max_top = (vh - ph - EDGE).max(EDGE);
    (left.clamp(EDGE, max_left), top.clamp(EDGE, max_top))
}

/// 任意内容的浮层（不是菜单）。`role="dialog" aria-modal="false"`：
/// 它不拦人，但 Esc / 点外照样收起。
///
/// 焦点契约：打开时进面板第一个可聚焦元素，关闭时回触发器。这跟菜单不同 ——
/// 菜单是「快速选一项」，气泡是「看/改一小块内容」，用户打开就是要操作它。
#[component]
pub fn Popover(
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] placement: PopoverPlacement,
    /// 与触发器之间的间距，默认 8。
    #[prop(optional)] offset: f64,
    /// 0 表示交给 CSS 的宽度（min 160 / max 320）。
    #[prop(optional)] width: u32,
    #[prop(into, optional)] label: String,
    trigger: Children,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let host = OverlayHost::use_or_provide();
    let wrap_ref = NodeRef::<html::Span>::new();
    let panel_ref = NodeRef::<html::Div>::new();
    let pos = RwSignal::new((0.0f64, 0.0f64));
    let ready = RwSignal::new(false);
    let layer = RwSignal::new(None::<OverlayId>);
    let return_to = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
    // 点外关闭不抢焦点（用户已经去别处了）；Esc / 程序关闭才归还触发器。
    let restore = StoredValue::new_local(true);
    let offset = if offset == 0.0 { EDGE } else { offset };
    let label = if label.is_empty() {
        "浮层".to_string()
    } else {
        label
    };

    let is_open = move || open.get().unwrap_or(false);
    let close = move || {
        if let Some(cb) = on_close {
            cb.run(());
        }
    };

    // 焦点归属：先看触发器里第一个可聚焦元素（通常是按钮），没有就退回当前焦点。
    let trigger_focus = move || {
        wrap_ref
            .get()
            .and_then(|w| first_focusable(&w))
            .or_else(active_element)
    };

    Effect::new(move |_| {
        if is_open() {
            let Some(trigger_el) = wrap_ref.get() else { return };
            if layer.get_untracked().is_none() {
                return_to.set_value(trigger_focus());
                let id = host.push(
                    OverlayKind::Popover,
                    Callback::new(move |_| {
                        if let Some(cb) = on_close {
                            cb.run(());
                        }
                    }),
                );
                layer.set(Some(id));
                restore.set_value(true);
            }
            if outside.with_value(|v| v.is_none()) {
                let panel_key = panel_ref;
                let wrap_key = wrap_ref;
                if let Some(listener) = listen_document("pointerdown", move |ev| {
                    let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    else {
                        return;
                    };
                    let inside_panel = panel_key
                        .get()
                        .map(|p| p.contains(Some(el.unchecked_ref::<web_sys::Node>())))
                        .unwrap_or(false);
                    let inside_trigger = wrap_key
                        .get()
                        .map(|w| w.contains(Some(el.unchecked_ref::<web_sys::Node>())))
                        .unwrap_or(false);
                    if !inside_panel && !inside_trigger {
                        restore.set_value(false);
                        close();
                    }
                }) {
                    outside.set_value(Some(listener));
                }
            }

            // 定位要等面板有真实尺寸：打开前它是 display:none，量到的是 0。
            // 先藏一帧（ready=false），量完再显形，避免先出现在错位置再跳一下。
            ready.set(false);
            let placement = placement;
            schedule(move || {
                let Some(panel) = panel_ref.get() else { return };
                let size = (panel.offset_width() as f64, panel.offset_height() as f64);
                let r = trigger_el.get_bounding_client_rect();
                pos.set(compute_pos(placement, &r, size, offset));
                ready.set(true);
                schedule(move || {
                    if let Some(panel) = panel_ref.get() {
                        match first_focusable(&panel) {
                            Some(el) => {
                                let _ = el.focus();
                            }
                            None => {
                                let _ = panel.focus();
                            }
                        }
                    }
                });
            });
        } else {
            if let Some(id) = layer.get_untracked() {
                host.remove(id);
                layer.set(None);
            }
            outside.set_value(None);
            ready.set(false);
            if restore.get_value() {
                if let Some(el) = return_to.get_value() {
                    schedule(move || {
                        if el.is_connected() {
                            let _ = el.focus();
                        }
                    });
                }
            }
            restore.set_value(true);
            return_to.set_value(None);
        }
    });

    on_cleanup(move || {
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    let panel_style = move || {
        if !is_open() {
            return "display:none".to_string();
        }
        let z = layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Popover.base_z());
        if !ready.get() {
            // 藏起来但保留布局：visibility 不影响测量，display:none 会。
            return format!("visibility:hidden;left:0;top:0;z-index:{z}");
        }
        let (left, top) = pos.get();
        let w = if width > 0 {
            format!(";width:{width}px")
        } else {
            String::new()
        };
        format!("left:{left}px;top:{top}px;z-index:{z}{w}")
    };

    view! {
        <span style="display:inline-flex" node_ref=wrap_ref>{trigger()}</span>
        <div
            // `.c-popover` 只给定位/圆角/内边距，材质在 `.c-glass-sheet` 那一组里 ——
            // 少写这一半，浮层就是一块透明文字（旧库也是两个类一起挂）。
            class="c-popover c-glass-sheet"
            role="dialog"
            aria-modal="false"
            aria-label=label
            tabindex="-1"
            node_ref=panel_ref
            style=panel_style
        >
            {children.map(|c| c())}
        </div>
    }
}
