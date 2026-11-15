use std::cell::Cell;
use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::base::anchor::{self, AnchorSpec, AnchorWatch, Placement, Side};
use crate::base::overlay::{
    active_element, first_focusable, listen_document, listen_window, schedule, GlobalListener,
    OverlayHost, OverlayId, OverlayKind,
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

/// 量面板的「内容高度」：不受当前 `max-height` 影响。
///
/// 未定位的第一帧面板是 `visibility:hidden` 而不是 `display:none`，所以这里能直接量；
/// 之前若被 `apply` 截断过，先把上限临时放开再还原。
fn content_height(panel: &web_sys::HtmlElement) -> f64 {
    let style = panel.style();
    let prev = style.get_property_value("max-height").unwrap_or_default();
    let _ = style.set_property("max-height", "none");
    let h = panel.scroll_height() as f64;
    let _ = style.set_property("max-height", &prev);
    h
}

/// 把一段长度夹进视口：左/上边界是 `margin`，右/下边界留出同样的缝。
fn clamp_axis(value: f64, size: f64, view: f64, margin: f64) -> f64 {
    let max = (view - size - margin).max(margin);
    value.clamp(margin, max)
}

/// 六向定位。
///
/// `BottomStart` 完全交给共享层；其余五个方向共享层只负责给出宽度与垂直翻转的基准，
/// 横向对齐（End / 侧向）与「声明在上」由这里补齐。所有结果都夹进视口。
fn place_popover(
    anchor_el: &web_sys::Element,
    panel: &web_sys::HtmlElement,
    spec: AnchorSpec,
    placement: PopoverPlacement,
) -> Placement {
    // 共享层先量宽度，并按「下方优先」算一次垂直结果（BottomStart 直接用它）。
    let base = anchor::place(anchor_el, panel, spec);
    let a = anchor_el.get_bounding_client_rect();
    let (vw, vh) = viewport();
    let margin = spec.margin;
    let width = base.width;

    match placement {
        PopoverPlacement::BottomStart => base,
        PopoverPlacement::BottomEnd => Placement {
            left: clamp_axis(a.right() - width, width, vw, margin),
            ..base
        },
        PopoverPlacement::TopStart | PopoverPlacement::TopEnd => {
            // 声明在上：上方放不下、且下方更宽裕时才翻下去。
            let content = content_height(panel);
            let below = vh - (a.bottom() + spec.gap) - margin;
            let above = a.top() - spec.gap - margin;
            let side = if spec.flip && content > above && below > above {
                Side::Below
            } else {
                Side::Above
            };
            let avail = if side == Side::Above { above } else { below };
            let max_height = avail.min(spec.max_height).max(0.0);
            let height = content.min(max_height);
            let top = if side == Side::Above {
                a.top() - spec.gap - height
            } else {
                a.bottom() + spec.gap
            };
            let left = if placement == PopoverPlacement::TopEnd {
                clamp_axis(a.right() - width, width, vw, margin)
            } else {
                base.left
            };
            Placement {
                side,
                left,
                top,
                width,
                max_height,
            }
        }
        PopoverPlacement::RightStart | PopoverPlacement::LeftStart => {
            let content = content_height(panel);
            let mut left = if placement == PopoverPlacement::RightStart {
                a.right() + spec.gap
            } else {
                a.left() - width - spec.gap
            };
            // 横向放不下就翻到另一侧，再夹进视口 —— 只夹取会让面板压住触发器。
            if left + width > vw - margin {
                left = a.left() - width - spec.gap;
            } else if left < margin {
                left = a.right() + spec.gap;
            }
            left = clamp_axis(left, width, vw, margin);
            let max_height = spec.max_height.min((vh - margin * 2.0).max(0.0));
            let height = content.min(max_height);
            let top = clamp_axis(a.top(), height, vh, margin);
            let side = if top >= a.top() {
                Side::Below
            } else {
                Side::Above
            };
            Placement {
                side,
                left,
                top,
                width,
                max_height,
            }
        }
    }
}

/// 面板尺寸变化的观察者（`ResizeObserver` 的最小封装）。
///
/// `控件/Cargo.toml` 的 web-sys 特性表是刻意收窄的，这里用 `Reflect` 取构造器而不是
/// 加特性；环境不支持时静默退化，只保留滚动 / resize 重算。
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

/// 非 `BottomStart` 的补充跟随。
///
/// `anchor::watch` 是统一的跟随链路（滚动 / resize / 面板自变高 + `on_gone`），但它只会
/// 「贴下方或翻上方、左对齐」。End 对齐与左右侧向需要在这条链路之后再修正一次：
/// 监听器按注册顺序在同一个 rAF 里跑，后注册的这一次覆盖前者，不会看到中间态。
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
        on_gone: Option<Rc<dyn Fn()>>,
    ) -> Self {
        let queued = Rc::new(Cell::new(false));
        let pump: Rc<dyn Fn()> = {
            let anchor_el = anchor_el.clone();
            let panel = panel.clone();
            let queued = Rc::clone(&queued);
            let apply = Rc::clone(&apply);
            let on_gone = on_gone.clone();
            Rc::new(move || {
                if queued.replace(true) {
                    return;
                }
                let anchor_el = anchor_el.clone();
                let panel = panel.clone();
                let queued = Rc::clone(&queued);
                let apply = Rc::clone(&apply);
                let on_gone = on_gone.clone();
                schedule(move || {
                    queued.set(false);
                    if !anchor_el.is_connected() || !panel.is_connected() {
                        return;
                    }
                    if let Some(cb) = &on_gone {
                        let a = anchor_el.get_bounding_client_rect();
                        let (_, vh) = viewport();
                        if a.bottom() <= 0.0 || a.top() >= vh {
                            cb();
                            return;
                        }
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

/// 跟随句柄：共享 `AnchorWatch` 管生命周期，`AlignFix` 只在需要时补一次对齐。
struct PopoverFollow {
    _anchor: AnchorWatch,
    _fix: Option<AlignFix>,
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
    let layer = RwSignal::new(None::<OverlayId>);
    let return_to = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
    // 跟随句柄：关闭时 take 掉，Drop 自动解绑滚动 / resize / 尺寸监听。
    let watch = StoredValue::new_local(None::<PopoverFollow>);
    // 点外关闭不抢焦点（用户已经去别处了）；Esc / 程序关闭才归还触发器。
    let restore = StoredValue::new_local(true);
    let offset = if offset == 0.0 { EDGE } else { offset };
    // Portal 的 children 必须是 `Fn`：`Children` 是 FnOnce，只能取一次后交给面板。
    let children_store = StoredValue::new_local(children);
    let label_store = StoredValue::new_local(if label.is_empty() {
        "浮层".to_string()
    } else {
        label
    });

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
            let Some(trigger_el) = wrap_ref.get().map(|w| w.unchecked_into::<web_sys::Element>())
            else {
                return;
            };
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
            // 点外关闭要同时认**触发器和面板**两个子树：面板已经 Portal 到 body，
            // 不在组件 DOM 里，只查触发器会把「点面板内部」判成点外。
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

            // 先藏一帧：面板的 style 闭包写的是 `left:-10000px;visibility:hidden`，
            // 等它落定再量高度、摆位置，用户不会看到从旧坐标跳一下。
            let spec = AnchorSpec {
                gap: offset,
                min_width: if width > 0 { width as f64 } else { 0.0 },
                scroll: false,
                ..AnchorSpec::default()
            };
            schedule(move || {
                if !is_open() {
                    return;
                }
                let Some(panel) = panel_ref.get() else { return };
                if !trigger_el.is_connected() || !panel.is_connected() {
                    return;
                }
                let placed = place_popover(&trigger_el, &panel, spec, placement);
                anchor::apply(&panel, placed, spec);

                // 定位完成、面板已显形之后再搬焦点：第一帧它是 visibility:hidden，
                // 直接 focus 会被浏览器忽略。
                schedule(move || {
                    if !is_open() {
                        return;
                    }
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

                // 锚点被滚出视口就收起：留一个挂在屏幕外的面板没有意义。
                let on_gone: Rc<dyn Fn()> = Rc::new(move || {
                    // 关闭后才跑到的 rAF 不要再回调一次 on_close。
                    if is_open() {
                        close();
                    }
                });
                let panel_el: web_sys::HtmlElement = panel.into();
                let (anchor_watch, fix) = if placement == PopoverPlacement::BottomStart {
                    (
                        anchor::watch(trigger_el.clone(), panel_el.clone(), spec, Some(on_gone)),
                        None,
                    )
                } else {
                    // 先挂共享 watch，再挂 AlignFix：同一事件里监听器按注册顺序执行，
                    // 后注册的这一份才能覆盖共享层的「贴下方 + 左对齐」。
                    let anchor_watch = anchor::watch(trigger_el.clone(), panel_el.clone(), spec, None);
                    // 共享层只认「下方/上方 + 左对齐」，End 与侧向在这里补正；
                    // 它的 on_gone 交给 AlignFix，避免用「贴下方」的坐标误判锚点已滚出。
                    let apply = Rc::new(move |a: &web_sys::Element, p: &web_sys::HtmlElement| {
                        if !is_open() {
                            return;
                        }
                        let placed = place_popover(a, p, spec, placement);
                        anchor::apply(p, placed, spec);
                    });
                    let fix = AlignFix::new(trigger_el, panel_el, apply, Some(on_gone));
                    (anchor_watch, Some(fix))
                };
                watch.set_value(Some(PopoverFollow {
                    _anchor: anchor_watch,
                    _fix: fix,
                }));
            });
        } else {
            if let Some(id) = layer.get_untracked() {
                host.remove(id);
                layer.set(None);
            }
            outside.set_value(None);
            watch.set_value(None);
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
        watch.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    view! {
        <span style="display:inline-flex" node_ref=wrap_ref>{trigger()}</span>

        // 面板必须到 body 下：`.c-field` / 卡片上的 backdrop-filter、transform
        // 会给 position:fixed 造包含块，面板会按祖先坐标摆、跟着容器滚。
        <Portal>
            <div
                // `.c-popover` 只给定位/圆角/内边距，材质在 `.c-glass-sheet` 那一组里 ——
                // 少写这一半，浮层就是一块透明文字（旧库也是两个类一起挂）。
                class="c-popover c-glass-sheet"
                role="dialog"
                aria-modal="false"
                aria-label=label_store.get_value()
                tabindex="-1"
                node_ref=panel_ref
                style=move || {
                    if !is_open() {
                        return "display:none".to_string();
                    }
                    let z = layer
                        .get()
                        .map(|id| host.z(id))
                        .unwrap_or(OverlayKind::Popover.base_z());
                    let w = if width > 0 {
                        format!(";width:{width}px")
                    } else {
                        String::new()
                    };
                    // 定位前先藏到视口外；`anchor::apply` 会逐条改内联样式并显形。
                    format!("z-index:{z};position:fixed;left:-10000px;top:0px;visibility:hidden{w}")
                }
            >
                // 面板常驻（关闭时 display:none）：`children` 是 FnOnce，不能每次打开重建。
                {children_store
                    .try_update_value(|c| c.take())
                    .flatten()
                    .map(|c| c())}
            </div>
        </Portal>
    }
}
