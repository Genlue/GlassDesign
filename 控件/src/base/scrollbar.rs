//! 覆盖式滚动条。
//!
//! 为什么需要它：`base.css` 里那套原生滚动条有两个硬伤 ——
//!
//! 1. **`*:hover::-webkit-scrollbar-thumb` 不成立。** Chromium 只把 `:hover`
//!    解析成「滚动条部件自身被悬停」，指针落在内容区或轨道空白处都不触发。
//!    结果是拇指必须被精确压住才看得见，`base.css` 里那句「指针进入该滚动容器
//!    就浮现」的注释与实现不符。
//! 2. **原生滚动条占 16px 布局宽度。** 内容列被挤窄 16px，左右内边距对不上。
//!
//! `base.css` 里其实早就写好了 `.c-scroller` + `.c-overlay-scroll` 这一套覆盖式
//! 样式，注释也说了「由 scrollbar.js 接管」—— 但那个脚本从来没被写出来。
//! 这里补上驱动，样式仍然全部来自 CSS，本模块不新增任何视觉。
//!
//! 挂载点很关键：覆盖层**不能**放进滚动容器自己肚子里。滚动容器里的
//! `position: absolute` 子元素会跟着内容一起滚走，拇指会消失。所以它挂在
//! **不滚动的定位祖先**上（`.l-stage` → `.l-main`，`.l-nav__body` → `.l-nav`），
//! 再由 `top` 补上滚动容器相对该祖先的偏移。

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

/// 拇指最短高度。与 `base.css` 里 `::-webkit-scrollbar-thumb` 的 `min-height` 对齐。
const MIN_THUMB: f64 = 44.0;
/// 轨道相对容器上下各留的白
const INSET: f64 = 4.0;
/// 停止交互后多久淡出
const HIDE_MS: f64 = 900.0;
/// 心跳间隔：负责淡出计时，以及内容变高后重算拇指长度
const TICK_MS: i32 = 150;

/// 句柄。`Drop` 时摘掉监听、清掉定时器 —— 导航栏会随窗口状态重建，
/// 不摘就是每次重建漏一批闭包和一个 interval。
pub struct ScrollbarHandle {
    detach: Option<Box<dyn FnOnce()>>,
}

impl Drop for ScrollbarHandle {
    fn drop(&mut self) {
        if let Some(f) = self.detach.take() {
            f();
        }
    }
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// 给一个滚动容器挂上覆盖式滚动条。
///
/// * `scroller` —— 真正滚动的元素（`.l-stage` / `.l-nav__body`）
/// * `host` —— 不滚动、且是定位祖先的元素（`.l-main` / `.l-nav`），覆盖层挂在它下面
pub fn attach(scroller: &Element, host: &Element) -> ScrollbarHandle {
    // 已经挂过就不要再挂（组件可能因父级重渲染被多次 enhance）
    if host
        .query_selector(".c-overlay-scroll")
        .ok()
        .flatten()
        .is_some()
    {
        return ScrollbarHandle { detach: None };
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return ScrollbarHandle { detach: None };
    };

    // 原生滚动条宽度归零（`.c-scroller` 负责 scrollbar-width 与 ::-webkit-scrollbar）
    let _ = scroller.class_list().add_1("c-scroller");

    let Ok(track) = doc.create_element("div") else {
        return ScrollbarHandle { detach: None };
    };
    track.set_class_name("c-overlay-scroll");
    let _ = track.set_attribute("data-on", "0");
    let Ok(thumb) = doc.create_element("div") else {
        return ScrollbarHandle { detach: None };
    };
    thumb.set_class_name("c-overlay-scroll__thumb");
    if track.append_child(&thumb).is_err() || host.append_child(&track).is_err() {
        return ScrollbarHandle { detach: None };
    }

    let visible = Rc::new(Cell::new(false));
    let last_active = Rc::new(Cell::new(0.0f64));
    // reflow 算出来的两个数，paint 直接拿去用（避免滚动时反复量布局）
    let travel = Rc::new(Cell::new(0.0f64));
    let max_scroll = Rc::new(Cell::new(1.0f64));

    /* ---------------- 几何：只在挂载 / 尺寸变化时算 ---------------- */

    let reflow: Rc<dyn Fn() -> bool> = {
        let scroller = scroller.clone();
        let host = host.clone();
        let track = track.clone();
        let thumb = thumb.clone();
        let travel = travel.clone();
        let max_scroll = max_scroll.clone();
        Rc::new(move || {
            let client_h = scroller.client_height() as f64;
            let scroll_h = scroller.scroll_height() as f64;
            if client_h < 40.0 || scroll_h <= client_h + 1.0 {
                let _ = track.set_attribute("data-on", "0");
                travel.set(0.0);
                return false;
            }

            // 滚动容器相对 host 的纵向偏移：导航栏的 head 那一行就是靠它让开的
            let dy = scroller.get_bounding_client_rect().top()
                - host.get_bounding_client_rect().top();
            let track_h = (client_h - INSET * 2.0).max(0.0);
            let thumb_h = (client_h * client_h / scroll_h).clamp(MIN_THUMB.min(track_h), track_h);

            travel.set((track_h - thumb_h).max(0.0));
            max_scroll.set((scroll_h - client_h).max(1.0));

            if let Some(h) = track.dyn_ref::<HtmlElement>() {
                let s = h.style();
                let _ = s.set_property("top", &format!("{}px", dy + INSET));
                let _ = s.set_property("height", &format!("{track_h}px"));
            }
            if let Some(h) = thumb.dyn_ref::<HtmlElement>() {
                let _ = h.style().set_property("height", &format!("{thumb_h}px"));
            }
            true
        })
    };

    /* ---------------- 位置：滚动时每帧更新 ---------------- */

    let paint: Rc<dyn Fn()> = {
        let scroller = scroller.clone();
        let thumb = thumb.clone();
        let travel = travel.clone();
        let max_scroll = max_scroll.clone();
        Rc::new(move || {
            let y = (scroller.scroll_top() as f64 / max_scroll.get()) * travel.get();
            if let Some(h) = thumb.dyn_ref::<HtmlElement>() {
                let _ = h
                    .style()
                    .set_property("transform", &format!("translateY({y}px)"));
            }
        })
    };

    /* ---------------- 显示 / 淡出 ---------------- */

    let touch: Rc<dyn Fn()> = {
        let visible = visible.clone();
        let last_active = last_active.clone();
        let track = track.clone();
        let paint = paint.clone();
        let reflow = reflow.clone();
        Rc::new(move || {
            last_active.set(now_ms());
            if !visible.get() {
                // 挂载那一刻内容还没铺满，量出来的几何是过期的；
                // 每次从「看不见」变「看得见」都重新量一遍。
                if !reflow() {
                    visible.set(false);
                    return;
                }
                visible.set(true);
                let _ = track.set_attribute("data-on", "1");
            }
            paint();
        })
    };

    // 每个监听都把闭包**留在解绑器里**：`Closure::forget()` 会永久泄漏，
    // 而组件卸载时我们需要连闭包一起放掉。监听期间闭包必须活着，正好由它持有。
    let mut detachers: Vec<Box<dyn FnOnce()>> = Vec::new();

    // 指针进入容器就浮现（这条才是 base.css 想做但做不到的）
    {
        let touch = touch.clone();
        let cb = Closure::<dyn FnMut()>::new(move || touch());
        let _ = scroller.add_event_listener_with_callback("pointerenter", cb.as_ref().unchecked_ref());
        let target = scroller.clone();
        detachers.push(Box::new(move || {
            let _ = target.remove_event_listener_with_callback(
                "pointerenter",
                cb.as_ref().unchecked_ref(),
            );
        }));
    }
    // 兜底：指针本来就在容器里（组件挂载时、或者刚淡出又没离开）时
    // `pointerenter` 不会再触发，靠移动事件把它叫回来。守卫是一次 Cell 读，很便宜。
    {
        let touch = touch.clone();
        let visible = visible.clone();
        let cb = Closure::<dyn FnMut()>::new(move || {
            if !visible.get() {
                touch();
            }
        });
        let _ = scroller.add_event_listener_with_callback("pointermove", cb.as_ref().unchecked_ref());
        let target = scroller.clone();
        detachers.push(Box::new(move || {
            let _ =
                target.remove_event_listener_with_callback("pointermove", cb.as_ref().unchecked_ref());
        }));
    }
    // 滚动：更新位置并续期
    {
        let touch = touch.clone();
        let cb = Closure::<dyn FnMut()>::new(move || touch());
        let _ = scroller.add_event_listener_with_callback("scroll", cb.as_ref().unchecked_ref());
        let target = scroller.clone();
        detachers.push(Box::new(move || {
            let _ = target.remove_event_listener_with_callback("scroll", cb.as_ref().unchecked_ref());
        }));
    }
    // 窗口尺寸变化：几何要重算
    {
        let reflow = reflow.clone();
        let paint = paint.clone();
        let cb = Closure::<dyn FnMut()>::new(move || {
            let _ = reflow();
            paint();
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
        }
        detachers.push(Box::new(move || {
            if let Some(w) = web_sys::window() {
                let _ = w.remove_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
            }
        }));
    }

    /* ---------------- 拖动拇指 ---------------- */

    let dragging = Rc::new(Cell::new(false));
    {
        let (start_y, start_scroll, ratio) = (
            Rc::new(Cell::new(0.0f64)),
            Rc::new(Cell::new(0.0f64)),
            Rc::new(Cell::new(1.0f64)),
        );

        {
            let dragging = dragging.clone();
            let (start_y, start_scroll, ratio) =
                (start_y.clone(), start_scroll.clone(), ratio.clone());
            let scroller = scroller.clone();
            let thumb_el = thumb.clone();
            let touch = touch.clone();
            let (max_scroll, travel) = (max_scroll.clone(), travel.clone());
            let cb = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
                move |ev: web_sys::PointerEvent| {
                    ev.prevent_default();
                    ev.stop_propagation();
                    ratio.set(max_scroll.get() / travel.get().max(1.0));
                    dragging.set(true);
                    start_y.set(ev.client_y() as f64);
                    start_scroll.set(scroller.scroll_top() as f64);
                    let _ = thumb_el.set_pointer_capture(ev.pointer_id());
                    touch();
                },
            );
            let _ = thumb.add_event_listener_with_callback("pointerdown", cb.as_ref().unchecked_ref());
            let target = thumb.clone();
            detachers.push(Box::new(move || {
                let _ = target
                    .remove_event_listener_with_callback("pointerdown", cb.as_ref().unchecked_ref());
            }));
        }

        {
            let dragging = dragging.clone();
            let (start_y, start_scroll, ratio) =
                (start_y.clone(), start_scroll.clone(), ratio.clone());
            let scroller = scroller.clone();
            let cb = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
                move |ev: web_sys::PointerEvent| {
                    if !dragging.get() {
                        return;
                    }
                    ev.prevent_default();
                    let dy = ev.client_y() as f64 - start_y.get();
                    scroller.set_scroll_top((start_scroll.get() + dy * ratio.get()) as i32);
                },
            );
            let _ = thumb.add_event_listener_with_callback("pointermove", cb.as_ref().unchecked_ref());
            let target = thumb.clone();
            detachers.push(Box::new(move || {
                let _ = target
                    .remove_event_listener_with_callback("pointermove", cb.as_ref().unchecked_ref());
            }));
        }

        {
            let dragging = dragging.clone();
            let thumb_el = thumb.clone();
            let cb = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
                move |ev: web_sys::PointerEvent| {
                    if !dragging.get() {
                        return;
                    }
                    dragging.set(false);
                    let _ = thumb_el.release_pointer_capture(ev.pointer_id());
                },
            );
            let _ = thumb.add_event_listener_with_callback("pointerup", cb.as_ref().unchecked_ref());
            let target = thumb.clone();
            detachers.push(Box::new(move || {
                let _ = target
                    .remove_event_listener_with_callback("pointerup", cb.as_ref().unchecked_ref());
            }));
        }
    }

    /* ---------------- 心跳 ---------------- */

    {
        let (visible, last_active) = (visible.clone(), last_active.clone());
        let track = track.clone();
        let (reflow, paint) = (reflow.clone(), paint.clone());
        let cb = Closure::<dyn FnMut()>::new(move || {
            if !visible.get() {
                return;
            }
            if now_ms() - last_active.get() > HIDE_MS {
                visible.set(false);
                let _ = track.set_attribute("data-on", "0");
                return;
            }
            // 分块渲染会不断加长内容，拇指长度得跟着变
            let _ = reflow();
            paint();
        });
        let handle = web_sys::window().and_then(|w| {
            w.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                TICK_MS,
            )
            .ok()
        });
        detachers.push(Box::new(move || {
            if let (Some(w), Some(id)) = (web_sys::window(), handle) {
                w.clear_interval_with_handle(id);
            }
            drop(cb);
        }));
    }

    let _ = reflow();
    paint();

    ScrollbarHandle {
        detach: Some(Box::new(move || {
            for f in detachers {
                f();
            }
        })),
    }
}

/// 组件里用的一行式挂载：拿到元素 → 找定位祖先 → 挂上 → 把句柄交给当前 owner。
///
/// 用 `StoredValue::new_local` 而不是 `on_cleanup`：后者的闭包要求 `Send`，
/// 而句柄里握的是 DOM 元素和 `Closure`（都不是 `Send`）。
/// `StoredValue` 的值挂在当前 owner 的 arena 上，owner 销毁时自动 Drop。
pub fn enhance(scroller: Element) {
    let Some(host) = scroller.parent_element() else {
        return;
    };
    let _guard = StoredValue::new_local(attach(&scroller, &host));
}
