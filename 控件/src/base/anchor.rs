//! 锚定浮层的定位与跟随：贴锚点、越界翻转、视口夹取、滚动重算。
//!
//! 为什么单独一层：
//! 1. 坐标。面板一律 `position: fixed` 且通过 `Portal` 挂到 `body`，所以只能用
//!    **视口坐标**。以前各组件在自己的 DOM 里用 `position: fixed`，一旦某个祖先有
//!    `transform` / `backdrop-filter` / `filter`，那个祖先就成了包含块 —— 面板会按
//!    祖先的坐标摆，还会跟着容器滚，甚至为露出自己把整页滚走。
//! 2. 翻转。下方放不下就翻到上方；左右越界就夹回视口。这两件事都要先量面板高度，
//!    所以必须「先渲染、后定位」，不能像以前那样在打开的一瞬间拍脑袋定 top。
//! 3. 跟随。页面或内部容器滚动、窗口 resize、面板内容变高（过滤列表）时都要重算，
//!    否则面板会僵在原地。
//!
//! 约定：组件只负责「什么时候开」，定位与跟随一律走这里。

use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::base::overlay::{listen_window, schedule, GlobalListener};

/// 面板落在锚点的哪一侧。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Below,
    Above,
}

/// 一次定位的结果。`left/top` 是视口坐标（`position: fixed` 直接可用）。
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Placement {
    pub side: Side,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub max_height: f64,
}

/// 定位策略。默认值是「下拉菜单」这一档，其它形态按需覆盖。
#[derive(Clone, Copy, Debug)]
pub struct AnchorSpec {
    /// 锚点与面板之间的缝。
    pub gap: f64,
    /// 与锚点同宽（下拉/选单固定这么做，否则打字时宽度会跳）。
    pub match_width: bool,
    /// 与视口边缘的最小距离。
    pub margin: f64,
    /// 面板高度上限（即使视口还有空间也不超过它）。
    pub max_height: f64,
    /// 面板最小宽度（`match_width=false` 时生效）。
    pub min_width: f64,
    /// 超出可用高度时是否让面板内部滚动。气泡/提示不该滚，设 `false`。
    pub scroll: bool,
    /// 是否允许向上翻转。提示条这类「必须贴在锚点旁边」的可以关掉。
    pub flip: bool,
}

impl Default for AnchorSpec {
    fn default() -> Self {
        Self {
            gap: 6.0,
            match_width: false,
            margin: 8.0,
            max_height: 420.0,
            min_width: 0.0,
            scroll: true,
            flip: true,
        }
    }
}

impl AnchorSpec {
    /// 下拉菜单：与锚点同宽、贴下方、超长内部滚动。
    pub fn dropdown() -> Self {
        Self {
            match_width: true,
            ..Self::default()
        }
    }

    /// 自由气泡：宽度按内容，不滚动。
    pub fn bubble() -> Self {
        Self {
            scroll: false,
            max_height: f64::INFINITY,
            ..Self::default()
        }
    }
}

/// 量面板的「内容高度」：不受当前 `max-height` 影响。
///
/// `scroll_height` 会被 `max-height` 截断（看不到溢出内容），所以先把上限临时放开再量，
/// 量完立刻还原。这一次读写在同一个任务里，不会产生可见闪烁。
fn content_height(panel: &web_sys::HtmlElement) -> f64 {
    let style = panel.style();
    let prev = style.get_property_value("max-height").unwrap_or_default();
    let _ = style.set_property("max-height", "none");
    let h = panel.scroll_height() as f64;
    let _ = style.set_property("max-height", &prev);
    h
}

fn viewport() -> (f64, f64) {
    let Some(win) = web_sys::window() else {
        return (1024.0, 768.0);
    };
    // 用 documentElement 的客户区而不是 innerWidth：后者含滚动条宽度，
    // 拿它算右边界会让面板压到滚动条下面。
    let doc = win.document().and_then(|d| d.document_element());
    let w = doc
        .as_ref()
        .map(|d| d.client_width() as f64)
        .filter(|w| *w > 0.0)
        .unwrap_or_else(|| win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1024.0));
    let h = doc
        .as_ref()
        .map(|d| d.client_height() as f64)
        .filter(|h| *h > 0.0)
        .unwrap_or_else(|| win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(768.0));
    (w, h)
}

/// 算出面板该摆在哪。**不写样式**，方便调用方先决定要不要显示。
pub fn place(
    anchor: &web_sys::Element,
    panel: &web_sys::HtmlElement,
    spec: AnchorSpec,
) -> Placement {
    let (vw, vh) = viewport();
    let a = anchor.get_bounding_client_rect();

    let content = content_height(panel);
    let natural_w = panel.offset_width() as f64;
    let width = if spec.match_width {
        a.width()
    } else {
        natural_w.max(spec.min_width)
    };
    // 左右夹取：先算理想左边（与锚点左对齐），再夹进 [margin, vw - width - margin]。
    let max_left = (vw - width - spec.margin).max(spec.margin);
    let left = a.left().clamp(spec.margin, max_left);

    let below = vh - (a.bottom() + spec.gap) - spec.margin;
    let above = a.top() - spec.gap - spec.margin;
    // 下方够放就不翻；不够但有更多上方空间就翻上去。`flip=false` 时死守下方。
    let side = if spec.flip && content > below && above > below {
        Side::Above
    } else {
        Side::Below
    };
    let avail = if side == Side::Below { below } else { above };
    let max_height = avail.min(spec.max_height).max(0.0);
    let height = content.min(max_height);
    let top = match side {
        Side::Below => a.bottom() + spec.gap,
        Side::Above => a.top() - spec.gap - height,
    };

    Placement {
        side,
        left,
        top,
        width,
        max_height,
    }
}

/// 把定位结果写进内联样式。
pub fn apply(panel: &web_sys::HtmlElement, p: Placement, spec: AnchorSpec) {
    let style = panel.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("left", &format!("{:.2}px", p.left));
    let _ = style.set_property("top", &format!("{:.2}px", p.top));
    let _ = style.set_property("width", &format!("{:.2}px", p.width));
    // `max-height` 只在真的需要截断时写，避免给不滚动的气泡加出一条滚动条。
    if p.max_height.is_finite() && p.max_height < 10_000.0 {
        let _ = style.set_property("max-height", &format!("{:.2}px", p.max_height));
    } else {
        let _ = style.remove_property("max-height");
    }
    let _ = style.set_property("overflow-y", if spec.scroll { "auto" } else { "visible" });
    // 定位完成才显形：打开的第一帧用 `visibility:hidden` 藏住，避免从旧坐标跳一下。
    let _ = style.set_property("visibility", "visible");
    let _ = panel.set_attribute("data-side", if p.side == Side::Below { "below" } else { "above" });
}

/// 一次「量了再摆」。
pub fn place_and_apply(
    anchor: &web_sys::Element,
    panel: &web_sys::HtmlElement,
    spec: AnchorSpec,
) -> Placement {
    let p = place(anchor, panel, spec);
    apply(panel, p, spec);
    p
}

/// 跟随句柄。Drop 时解绑所有监听器 —— 放进 `StoredValue` 就能活到组件卸载。
pub struct AnchorWatch {
    _listeners: Vec<GlobalListener>,
    _resize_observer: Option<ResizeObserverHandle>,
}

/// 跟着锚点走：滚动（含内部容器，用 capture 才收得到）、resize、面板尺寸变化都重算。
///
/// `on_gone` 可选：锚点被滚出视口（上下都不在）时回调，通常用来关掉浮层。
/// 返回的句柄必须持有到浮层关闭，否则监听器立刻失效。
pub fn watch(
    anchor: web_sys::Element,
    panel: web_sys::HtmlElement,
    spec: AnchorSpec,
    // 锚点被滚出视口时调用（通常用来关掉浮层）。
    on_gone: Option<Rc<dyn Fn()>>,
) -> AnchorWatch {
    let anchor_scroll = anchor.clone();
    let panel_scroll = panel.clone();
    // rAF 合帧：滚动一次会来几十个事件，每次都量布局会掉帧。
    let queued = Rc::new(std::cell::Cell::new(false));
    let pump: Rc<dyn Fn()> = {
        let anchor = anchor_scroll;
        let panel = panel_scroll;
        let queued = Rc::clone(&queued);
        Rc::new(move || {
            if queued.replace(true) {
                return;
            }
            let anchor = anchor.clone();
            let panel = panel.clone();
            let queued = Rc::clone(&queued);
            let gone = on_gone.clone();
            schedule(move || {
                queued.set(false);
                if !anchor.is_connected() || !panel.is_connected() {
                    return;
                }
                let p = place_and_apply(&anchor, &panel, spec);
                // 锚点被滚出视口：留着一个挂在屏幕外的面板没有意义。
                if let Some(cb) = &gone {
                    let (_, vh) = viewport();
                    if p.top + p.max_height <= 0.0 || p.top >= vh {
                        cb();
                    }
                }
            });
        })
    };

    let mut listeners = Vec::new();
    // scroll 不冒泡，必须 capture=true 才能收到内部滚动容器的事件。
    if let Some(l) = listen_window("scroll", true, {
        let pump = pump.clone();
        move |_| pump()
    }) {
        listeners.push(l);
    }
    if let Some(l) = listen_window("resize", false, {
        let pump = pump.clone();
        move |_| pump()
    }) {
        listeners.push(l);
    }

    // 面板自身变高/变宽（例如过滤后条目变少）也要重摆一次。
    let observer = ResizeObserverHandle::new(panel.clone(), {
        let pump = pump.clone();
        move || pump()
    });

    AnchorWatch {
        _listeners: listeners,
        _resize_observer: observer,
    }
}

/// `ResizeObserver` 的最小封装。
///
/// 这里用 `Reflect` 取构造器而不是 `web_sys::ResizeObserver`：`控件/Cargo.toml` 的
/// web-sys 特性表是刻意收窄的（多个组件族共用），为这一处再加特性不值当。环境不支持
/// 时静默退化为「只在滚动/resize 时重算」。
struct ResizeObserverHandle {
    observer: JsValue,
    closure: Closure<dyn FnMut()>,
}

impl ResizeObserverHandle {
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

impl Drop for ResizeObserverHandle {
    fn drop(&mut self) {
        if let Ok(disconnect) = js_sys::Reflect::get(&self.observer, &JsValue::from_str("disconnect")) {
            if let Ok(f) = disconnect.dyn_into::<js_sys::Function>() {
                let _ = f.call0(&self.observer);
            }
        }
        let _ = &self.closure; // 明确：closure 必须活得比 observer 久
    }
}

/// 给「刚打开、还没定位」的面板一个不会闪的初始状态。
///
/// 定位是在 Effect 里跑的（要等 DOM 落定才能量高度），所以打开的第一帧会按旧样式
/// 画一次。把它挪到视口外 + 隐藏，等 `apply` 再显形，用户就看不到那一下跳动。
pub fn hide_until_placed(panel: &web_sys::HtmlElement) {
    let style = panel.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("left", "-10000px");
    let _ = style.set_property("top", "0px");
    let _ = style.set_property("visibility", "hidden");
}
