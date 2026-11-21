use std::cell::RefCell;
use std::rc::Rc;

use leptos::html;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::icon::Icon;
use crate::widgets::media_icons::media_icons;

/// 单行歌词条目。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
    pub translation: Option<String>,
}

const USER_SCROLL_HOLD_MS: f64 = 3000.0;
/// 临界阻尼弹簧参数（ζ ≈ 1.015）：起步干脆、尾段丝滑减速、无回弹抖动。
const SPRING_STIFFNESS: f64 = 105.0;
const SPRING_DAMPING: f64 = 20.8;

#[derive(Default)]
struct SpringState {
    running: bool,
    pos: f64,
    vel: f64,
    target: f64,
    last_ts: f64,
    raf_id: Option<i32>,
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

pub(crate) fn line_state(idx: usize, active: Option<usize>) -> &'static str {
    match active {
        Some(a) if idx == a => "active",
        Some(a) if idx < a => "past",
        Some(_) => "future",
        None => "future",
    }
}

pub(crate) fn line_distance_bucket(idx: usize, active: Option<usize>) -> &'static str {
    let dist = match active {
        Some(a) => idx.abs_diff(a),
        None => idx,
    };
    match dist {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        _ => "far",
    }
}

pub(crate) fn format_lyric_timestamp(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}

fn compute_target_scroll_top(
    container: &web_sys::HtmlDivElement,
    list: &web_sys::HtmlDivElement,
    idx: usize,
    anchor_ratio: f64,
) -> Option<f64> {
    let children = list.children();
    let el = children.item(idx as u32)?;
    let el = el.dyn_into::<web_sys::HtmlElement>().ok()?;

    let c_rect = container.get_bounding_client_rect();
    let e_rect = el.get_bounding_client_rect();
    if c_rect.height() <= 0.0 {
        return None;
    }

    let ratio = anchor_ratio.clamp(0.15, 0.85);
    let max_scroll = (container.scroll_height() as f64 - c_rect.height()).max(0.0);
    let current_top = container.scroll_top() as f64;
    let raw = current_top
        + (e_rect.top() - c_rect.top())
        + e_rect.height() * 0.5
        - c_rect.height() * ratio;
    Some(raw.clamp(0.0, max_scroll))
}

fn cancel_spring(spring: &Rc<RefCell<SpringState>>) {
    let mut s = spring.borrow_mut();
    s.running = false;
    s.vel = 0.0;
    if let Some(id) = s.raf_id.take() {
        if let Some(w) = web_sys::window() {
            let _ = w.cancel_animation_frame(id);
        }
    }
}

/// 滚动歌词视图组件。
///
/// 支持高亮当前行、按行距渐进景深弱化、双语翻译展示、
/// `active_index` 变化时以非线性临界阻尼弹簧平滑对齐到指定垂直锚点（`anchor_ratio`，默认 0.5），
/// 用户手动滚动时短暂暂停自动跟随并恢复全部行清晰可读，
/// 以及点击任意歌词行跳转播放进度（`on_seek`）。
#[component]
pub fn LyricView(
    #[prop(into)] lines: Signal<Vec<LyricLine>>,
    #[prop(into, optional)] active_index: MaybeProp<usize>,
    #[prop(into, optional)] show_translation: MaybeProp<bool>,
    /// 当前高亮行垂直对齐位置占歌词容器高度的比例（0.0 = 顶部，0.5 = 居中）。
    #[prop(into, optional)] anchor_ratio: MaybeProp<f64>,
    /// 外部布局/字号/行距变化时递增该值以触发重新平滑对齐。
    #[prop(into, optional)] recenter_trigger: MaybeProp<u64>,
    #[prop(optional)] on_seek: Option<Callback<u64>>,
    #[prop(into, optional)] empty_text: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let container_ref = NodeRef::<html::Div>::new();
    let list_ref = NodeRef::<html::Div>::new();
    let manual_until = StoredValue::new_local(0.0f64);
    let manual_seq = RwSignal::new(0u32);
    let manual_scrolling = RwSignal::new(false);
    let spring = StoredValue::new_local(Rc::new(RefCell::new(SpringState::default())));
    let raf_cb: StoredValue<Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>, LocalStorage> =
        StoredValue::new_local(Rc::new(RefCell::new(None)));

    let mut cls = String::from("c-lyric");
    if on_seek.is_some() {
        cls.push_str(" c-lyric--seekable");
    }
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let empty_msg = if empty_text.is_empty() {
        String::from("暂无歌词")
    } else {
        empty_text
    };

    let show_trans = move || show_translation.get().unwrap_or(true);
    let ratio_now = move || anchor_ratio.get().unwrap_or(0.5).clamp(0.15, 0.85);

    // 启动或重定向非线性弹簧滚动（速度连续，切行/快进无突兀跳变）
    let spring_scroll_to = move |target: f64| {
        let Some(container) = container_ref.get_untracked() else {
            return;
        };
        let spring_rc = spring.get_value();

        if prefers_reduced_motion() {
            cancel_spring(&spring_rc);
            container.scroll_to_with_x_and_y(0.0, target);
            return;
        }

        let already_running = {
            let mut s = spring_rc.borrow_mut();
            if s.running {
                s.target = target;
                true
            } else {
                let start = container.scroll_top() as f64;
                if (target - start).abs() < 0.5 {
                    container.scroll_to_with_x_and_y(0.0, target);
                    return;
                }
                s.pos = start;
                s.vel = 0.0;
                s.target = target;
                s.last_ts = now_ms();
                s.running = true;
                false
            }
        };

        if already_running {
            return;
        }

        let cb_slot = raf_cb.get_value();
        let cb_slot_next = Rc::clone(&cb_slot);
        let spring_step = Rc::clone(&spring_rc);

        *cb_slot.borrow_mut() = Some(Closure::new(move |ts: f64| {
            let Some(container) = container_ref.try_get_untracked().flatten() else {
                cancel_spring(&spring_step);
                return;
            };
            if !container.is_connected() {
                cancel_spring(&spring_step);
                return;
            }

            let (done, next_pos) = {
                let mut s = spring_step.borrow_mut();
                s.raf_id = None;
                if !s.running {
                    return;
                }
                let dt = ((ts - s.last_ts) / 1000.0).clamp(0.001, 0.032);
                s.last_ts = ts;

                // 两步半步积分，保证不同刷新率（60/120/240Hz）下曲线一致且稳定
                let h = dt * 0.5;
                for _ in 0..2 {
                    let accel = SPRING_STIFFNESS * (s.target - s.pos) - SPRING_DAMPING * s.vel;
                    s.vel += accel * h;
                    s.pos += s.vel * h;
                }

                let max_scroll = (container.scroll_height() as f64
                    - container.client_height() as f64)
                    .max(0.0);
                s.pos = s.pos.clamp(0.0, max_scroll);
                let target_clamped = s.target.clamp(0.0, max_scroll);
                s.target = target_clamped;

                let settled =
                    (target_clamped - s.pos).abs() < 0.35 && s.vel.abs() < 1.5;
                if settled {
                    s.pos = target_clamped;
                    s.vel = 0.0;
                    s.running = false;
                    (true, target_clamped)
                } else {
                    (false, s.pos)
                }
            };

            container.scroll_to_with_x_and_y(0.0, next_pos);

            if !done {
                if let Some(win) = web_sys::window() {
                    if let Some(cb) = cb_slot_next.borrow().as_ref() {
                        if let Ok(id) = win.request_animation_frame(cb.as_ref().unchecked_ref()) {
                            spring_step.borrow_mut().raf_id = Some(id);
                        }
                    }
                }
            }
        }));

        if let Some(win) = web_sys::window() {
            if let Some(cb) = cb_slot.borrow().as_ref() {
                if let Ok(id) = win.request_animation_frame(cb.as_ref().unchecked_ref()) {
                    spring_rc.borrow_mut().raf_id = Some(id);
                }
            }
        }
    };

    let align_to_active = move |ignore_manual: bool| {
        if !ignore_manual && now_ms() < manual_until.get_value() {
            return;
        }
        let idx = active_index.get_untracked().unwrap_or(0);
        let (Some(container), Some(list)) =
            (container_ref.get_untracked(), list_ref.get_untracked())
        else {
            return;
        };
        if let Some(target) =
            compute_target_scroll_top(&container, &list, idx, anchor_ratio.get_untracked().unwrap_or(0.5))
        {
            spring_scroll_to(target);
        }
    };

    // 切歌时清除手动滚动锁定并停止旧弹簧、回到顶部
    Effect::new(move |_| {
        let _ = lines.with(|l| l.len());
        manual_until.set_value(0.0);
        manual_scrolling.set(false);
        manual_seq.update(|n| *n = n.wrapping_add(1));
        cancel_spring(&spring.get_value());
        if let Some(container) = container_ref.get_untracked() {
            container.scroll_to_with_x_and_y(0.0, 0.0);
        }
    });

    // 当前高亮行 / 锚点比例 / 外部字号行距变化时，下一帧量好最新行高后以非线性弹簧平滑对齐
    Effect::new(move |_| {
        let active = active_index.get();
        let _ratio = ratio_now();
        let _recenter = recenter_trigger.get();
        let _trans = show_trans();
        let Some(_idx) = active else {
            return;
        };
        if now_ms() < manual_until.get_value() {
            return;
        }
        if let Some(win) = web_sys::window() {
            let cb = Closure::once_into_js(move || {
                align_to_active(false);
            });
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        } else {
            align_to_active(false);
        }
    });

    on_cleanup(move || {
        cancel_spring(&spring.get_value());
        *raf_cb.get_value().borrow_mut() = None;
    });

    let mark_manual_scroll = move || {
        cancel_spring(&spring.get_value());
        manual_until.set_value(now_ms() + USER_SCROLL_HOLD_MS);
        manual_scrolling.set(true);
        let token = manual_seq.get_untracked().wrapping_add(1);
        manual_seq.set(token);
        if let Some(win) = web_sys::window() {
            let cb = Closure::once_into_js(move || {
                if manual_seq.try_get_untracked() == Some(token) {
                    let _ = manual_scrolling.try_set(false);
                    align_to_active(true);
                }
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                (USER_SCROLL_HOLD_MS as i32) + 60,
            );
        }
    };

    view! {
        <div
            class=cls
            node_ref=container_ref
            data-scrolling=move || manual_scrolling.get().then_some("true")
            on:wheel=move |_| mark_manual_scroll()
            on:touchstart=move |_| mark_manual_scroll()
        >
            {move || {
                let all = lines.get();
                if all.is_empty() {
                    view! {
                        <div class="c-lyric__empty">
                            <span class="c-lyric__empty-icon">
                                <Icon name=media_icons::LYRICS size=28 />
                            </span>
                            <span>{empty_msg.clone()}</span>
                        </div>
                    }
                    .into_any()
                } else {
                    let rendered = all
                        .into_iter()
                        .enumerate()
                        .map(|(idx, line)| {
                            let time_ms = line.time_ms;
                            let time_label = format_lyric_timestamp(time_ms);
                            let text = line.text;
                            let translation = line.translation;
                            let state = move || line_state(idx, active_index.get());
                            let dist = move || line_distance_bucket(idx, active_index.get());

                            view! {
                                <div
                                    class="c-lyric__line"
                                    data-state=state
                                    data-dist=dist
                                    role=if on_seek.is_some() { Some("button") } else { None }
                                    tabindex=if on_seek.is_some() { Some("0") } else { None }
                                    on:click=move |_| {
                                        if let Some(cb) = on_seek {
                                            manual_until.set_value(0.0);
                                            manual_scrolling.set(false);
                                            manual_seq.update(|n| *n = n.wrapping_add(1));
                                            cb.run(time_ms);
                                        }
                                    }
                                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                                        if let Some(cb) = on_seek {
                                            let k = ev.key();
                                            if k == "Enter" || k == " " {
                                                ev.prevent_default();
                                                manual_until.set_value(0.0);
                                                manual_scrolling.set(false);
                                                manual_seq.update(|n| *n = n.wrapping_add(1));
                                                cb.run(time_ms);
                                            }
                                        }
                                    }
                                >
                                    <span class="c-lyric__time">{time_label}</span>
                                    <div class="c-lyric__content">
                                        <div class="c-lyric__text">{text}</div>
                                        {move || {
                                            if show_trans() {
                                                translation
                                                    .as_ref()
                                                    .filter(|t| !t.trim().is_empty())
                                                    .map(|t| view! { <div class="c-lyric__trans">{t.clone()}</div> })
                                            } else {
                                                None
                                            }
                                        }}
                                    </div>
                                </div>
                            }
                        })
                        .collect_view();

                    view! {
                        <div class="c-lyric__list" node_ref=list_ref>
                            {rendered}
                        </div>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{format_lyric_timestamp, line_distance_bucket, line_state};

    #[test]
    fn line_state_classifies_past_active_and_future() {
        assert_eq!(line_state(0, None), "future");
        assert_eq!(line_state(1, Some(3)), "past");
        assert_eq!(line_state(3, Some(3)), "active");
        assert_eq!(line_state(5, Some(3)), "future");
    }

    #[test]
    fn line_distance_buckets_progressively() {
        assert_eq!(line_distance_bucket(3, Some(3)), "0");
        assert_eq!(line_distance_bucket(2, Some(3)), "1");
        assert_eq!(line_distance_bucket(4, Some(3)), "1");
        assert_eq!(line_distance_bucket(1, Some(3)), "2");
        assert_eq!(line_distance_bucket(6, Some(3)), "3");
        assert_eq!(line_distance_bucket(7, Some(3)), "4");
        assert_eq!(line_distance_bucket(9, Some(3)), "far");
        assert_eq!(line_distance_bucket(0, None), "0");
        assert_eq!(line_distance_bucket(2, None), "2");
    }

    #[test]
    fn lyric_timestamp_formats_mm_ss() {
        assert_eq!(format_lyric_timestamp(0), "00:00");
        assert_eq!(format_lyric_timestamp(65_400), "01:05");
        assert_eq!(format_lyric_timestamp(600_000), "10:00");
    }
}
