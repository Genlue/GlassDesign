use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::anchor::{self, AnchorSpec, AnchorWatch};
use crate::base::icon::{Icon, IconName};
use crate::base::overlay::{
    listen_document, GlobalListener, OverlayHost, OverlayId, OverlayKind,
};

/* ==========================================================================
   日期 / 时间的整数运算

   本仓库不加依赖（`chrono` 会被 `控件/Cargo.toml` 的「零额外依赖」挡掉），
   所以日期一律用「epoch 天」整数算：`days_from_civil` / `civil_from_days`
   是 Howard Hinnant 的算法，不含时区、不含夏令时，只在两个方向上互逆。

   为什么不借 `js_sys::Date` 做加减：`new Date('2026-11-06')` 按 UTC 解析、
   再按本地时区读回，负偏移时区会整体前移一天 —— 这类「少一天」的 bug 在
   日期控件里最难查。只有「今天是几号」这一件事必须问系统时钟。
   ========================================================================== */

const WEEKDAYS: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 30,
    }
}

/// 公历 → 距 1970-01-01 的天数（可为负）
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m as i64 + 9) % 12; // 三月 = 0
    let doy = (153 * mp + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y } as i32, m, d)
}

fn add_days(y: i32, m: u32, d: u32, delta: i64) -> (i32, u32, u32) {
    civil_from_days(days_from_civil(y, m, d) + delta)
}

/// 跨月时把日夹到目标月长度：1 月 31 日 + 1 月 → 2 月 28/29 日
fn add_months(y: i32, m: u32, d: u32, delta: i32) -> (i32, u32, u32) {
    let total = y * 12 + (m as i32 - 1) + delta;
    let ny = total.div_euclid(12);
    let nm = total.rem_euclid(12) as u32 + 1;
    (ny, nm, d.min(days_in_month(ny, nm)))
}

/// 周一 = 0 … 周日 = 6
fn weekday_monday(y: i32, m: u32, d: u32) -> u32 {
    let days = days_from_civil(y, m, d);
    ((days + 3).rem_euclid(7)) as u32
}

fn date_key(y: i32, m: u32, d: u32) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

fn parse_date(raw: &str) -> Option<(i32, u32, u32)> {
    let raw = raw.trim();
    let mut parts = raw.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    // 只认定宽写法：字符串比较就是日期比较，前面 enabled() 靠这点省掉了逐段解析
    (date_key(y, m, d) == raw).then_some((y, m, d))
}

fn today() -> (i32, u32, u32) {
    let now = js_sys::Date::new_0();
    (
        now.get_full_year() as i32,
        now.get_month() + 1,
        now.get_date(),
    )
}

fn parse_hm(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    let (h, m) = raw.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// 下一帧再执行。Leptos 的声明式重建在微任务里落定，焦点必须在它之后搬。
fn schedule(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}

/* ==========================================================================
   日期 DatePicker
   ========================================================================== */

/// 日期选择。触发器 + 浮层面板，值是 `YYYY-MM-DD` 字符串。
///
/// 面板非模态：不入焦点陷阱、不绑 inert，只借 `OverlayHost` 拿 z 与 Esc。
/// 网格里的焦点是「roving tabindex」—— 整张网格只有一个 `tabindex="0"`，
/// 移动靠手动 `.focus()`。给 42 个格子都设 `tabindex=0` 会让 Tab 变成酷刑。
#[component]
pub fn DatePicker(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(into, optional)] min: String,
    #[prop(into, optional)] max: String,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] placeholder: String,
    /// 面板底部是否给「清除」
    #[prop(default = true)] clearable: bool,
    #[prop(optional)] disabled: bool,
    /// 打开时永远先落在今天，而不是已选值
    #[prop(optional)] open_to_today: bool,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    let host = OverlayHost::use_or_provide();
    let trigger_ref = NodeRef::<html::Button>::new();
    let panel_ref = NodeRef::<html::Div>::new();

    let open = RwSignal::new(false);
    let layer = RwSignal::new(None::<OverlayId>);
    // 跟随句柄与点外监听：关闭时一起置 None，Drop 自动解绑。
    let watch = StoredValue::new_local(None::<AnchorWatch>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
    let view = RwSignal::new({
        let t = today();
        (t.0, t.1)
    });
    let focus_day = RwSignal::new(today());

    // 边界存成 Copy 句柄：下面的闭包要能被复制进 42 个格子
    let lo = StoredValue::new(min.trim().to_string());
    let hi = StoredValue::new(max.trim().to_string());
    let today_key = StoredValue::new({
        let t = today();
        date_key(t.0, t.1, t.2)
    });

    let enabled = move |key: &str| {
        let lo = lo.get_value();
        let hi = hi.get_value();
        (lo.is_empty() || key >= lo.as_str()) && (hi.is_empty() || key <= hi.as_str())
    };

    // 越界时夹到边界，而不是把焦点丢在一格灰线上：键盘用户按 → 应该一直有落点
    let clamp_day = move |y: i32, m: u32, d: u32| -> (i32, u32, u32) {
        let key = date_key(y, m, d);
        let lo = lo.get_value();
        if !lo.is_empty() && key.as_str() < lo.as_str() {
            if let Some(v) = parse_date(&lo) {
                return v;
            }
        }
        let hi = hi.get_value();
        if !hi.is_empty() && key.as_str() > hi.as_str() {
            if let Some(v) = parse_date(&hi) {
                return v;
            }
        }
        (y, m, d)
    };

    let current_key = move || {
        let raw = value.get().unwrap_or_default();
        parse_date(&raw).map(|(y, m, d)| date_key(y, m, d)).unwrap_or_default()
    };

    let restore = move |key: String| {
        schedule(move || {
            let Some(panel) = panel_ref.get() else { return };
            let sel = format!(".c-datepicker__day[data-day=\"{key}\"]");
            if let Ok(Some(el)) = panel.query_selector(&sel) {
                if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
        });
    };

    // 焦点是不是还在网格里：翻月的按钮被鼠标点过之后，不该把焦点抢回格子
    let in_grid = move || {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .map(|el| el.class_list().contains("c-datepicker__day"))
            .unwrap_or(false)
    };

    let close = move || {
        // 先收起、再解绑：面板的 style 闭包依赖 open，先关掉能保证随后的
        // 样式/栈变化不会擦掉已经写好的坐标。
        open.set(false);
        watch.set_value(None);
        outside.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
        // 关掉之后焦点回到触发器：键盘用户继续往下走
        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    };

    let pick = move |y: i32, m: u32, d: u32| {
        let key = date_key(y, m, d);
        if !enabled(&key) {
            return;
        }
        close();
        if let Some(cb) = on_change {
            cb.run(key);
        }
    };

    let clear = move || {
        close();
        if let Some(cb) = on_change {
            cb.run(String::new());
        }
    };

    let open_panel = move || {
        if disabled || layer.get_untracked().is_some() {
            return;
        }
        let now = today();
        let seed = if open_to_today {
            now
        } else {
            parse_date(&current_key()).unwrap_or(now)
        };
        let seed = clamp_day(seed.0, seed.1, seed.2);
        view.set((seed.0, seed.1));
        focus_day.set(seed);
        let id = host.push(OverlayKind::Popover, Callback::new(move |_| close()));
        layer.set(Some(id));
        open.set(true);
    };

    let toggle = move || {
        if open.get_untracked() {
            close();
        } else {
            open_panel();
        }
    };

    // 翻月：跟着「焦点日」走，而不是跟着视图走 —— 连点两次下一页才会翻两个月
    let shift = move |dir: i32| {
        let (fy, fm, fd) = focus_day.get_untracked();
        let (ty, tm, td) = add_months(fy, fm, fd, dir);
        let (ty, tm, td) = clamp_day(ty, tm, td);
        focus_day.set((ty, tm, td));
        view.set((ty, tm));
        if in_grid() {
            restore(date_key(ty, tm, td));
        }
    };

    // 目标月里一天可选都没有 → 导航禁用。整片灰的月份不值得让用户翻过去。
    let can_shift = move |dir: i32| {
        let (vy, vm) = view.get_untracked();
        let (ty, tm, _) = add_months(vy, vm, 1, dir);
        let first = date_key(ty, tm, 1);
        let last = date_key(ty, tm, days_in_month(ty, tm));
        let lo = lo.get_value();
        let hi = hi.get_value();
        (lo.is_empty() || last.as_str() >= lo.as_str())
            && (hi.is_empty() || first.as_str() <= hi.as_str())
    };

    let on_day_key = move |ev: web_sys::KeyboardEvent, y: i32, m: u32, d: u32| {
        if disabled {
            return;
        }
        let target = match ev.key().as_str() {
            "ArrowLeft" => Some(add_days(y, m, d, -1)),
            "ArrowRight" => Some(add_days(y, m, d, 1)),
            "ArrowUp" => Some(add_days(y, m, d, -7)),
            "ArrowDown" => Some(add_days(y, m, d, 7)),
            "PageUp" => Some(add_months(y, m, d, -1)),
            "PageDown" => Some(add_months(y, m, d, 1)),
            "Home" => Some(add_days(y, m, d, -(weekday_monday(y, m, d) as i64))),
            "End" => Some(add_days(y, m, d, 6 - weekday_monday(y, m, d) as i64)),
            _ => None,
        };
        if let Some((ty, tm, td)) = target {
            ev.prevent_default();
            let (ty, tm, td) = clamp_day(ty, tm, td);
            view.set((ty, tm));
            focus_day.set((ty, tm, td));
            restore(date_key(ty, tm, td));
            return;
        }
        if ev.key() == "Enter" || ev.key() == " " {
            ev.prevent_default();
            pick(y, m, d);
        }
    };

    let layer_z = move || {
        layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Popover.base_z())
    };

    // 打开后定位：DOM 落定 → 量 → 摆 → 跟随。
    // 这里再等一帧（rAF），是因为面板的 style 闭包（负责第一帧的
    // `visibility:hidden;left:-10000px`）在本 effect 之后才生效 —— 同一帧里
    // 先 apply 会被它整条 style 覆盖回视口外。等一帧后 style 已落定，apply 才留得住。
    Effect::new(move |_| {
        let anchor_el = trigger_ref
            .get()
            .map(|b| b.unchecked_into::<web_sys::Element>());
        let panel = panel_ref.get();
        if !open.get() {
            watch.set_value(None);
            outside.set_value(None);
            return;
        }
        let (Some(anchor_el), Some(panel)) = (anchor_el, panel) else {
            // Portal 还没挂好：NodeRef 是响应式的，挂上后会再跑一遍。
            return;
        };

        // 点外关闭必须同时认**触发器和面板**两个子树：面板已经 Portal 到 body，
        // 不在组件 DOM 里了，只查 root 会把「点面板内部」判成点外（日期格点不动）。
        if outside.with_value(|v| v.is_none()) {
            let trigger_key = trigger_ref;
            let panel_key = panel_ref;
            if let Some(listener) = listen_document("pointerdown", move |ev| {
                let Some(target) = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                let node: &web_sys::Node = target.unchecked_ref();
                let inside_trigger = trigger_key
                    .get()
                    .map(|t| t.contains(Some(node)))
                    .unwrap_or(false);
                let inside_panel = panel_key
                    .get()
                    .map(|p| p.contains(Some(node)))
                    .unwrap_or(false);
                if !inside_trigger && !inside_panel {
                    close();
                }
            }) {
                outside.set_value(Some(listener));
            }
        }

        schedule(move || {
            // 这一帧里可能已经关了（快速开→关）：别再建一个新的跟随句柄。
            if !open.get_untracked() {
                return;
            }
            if !anchor_el.is_connected() || !panel.is_connected() {
                return;
            }
            // 面板固定 236 宽、不出滚动条；宽度不跟 148 宽的触发器走。
            let spec = AnchorSpec {
                match_width: false,
                min_width: 236.0,
                scroll: false,
                ..AnchorSpec::bubble()
            };
            anchor::place_and_apply(&anchor_el, &panel, spec);
            // 定位完成、面板已显形之后再搬焦点：第一帧它是 visibility:hidden，
            // 直接 focus 会被浏览器忽略。焦点落到已选日的同时也把它滚进视口。
            let (y, m, d) = focus_day.get_untracked();
            restore(date_key(y, m, d));
            // 锚点被滚出视口就收起：留一个挂在屏幕外的面板没有意义。
            let on_gone: Rc<dyn Fn()> = Rc::new(move || close());
            watch.set_value(Some(anchor::watch(anchor_el, panel.into(), spec, Some(on_gone))));
        });
    });

    let aria_label = if label.is_empty() {
        "日期".to_string()
    } else {
        label.clone()
    };
    let blank = if placeholder.is_empty() {
        "选择日期".to_string()
    } else {
        placeholder.clone()
    };

    on_cleanup(move || {
        watch.set_value(None);
        outside.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    view! {
        <div class="c-datepicker" data-empty=move || current_key().is_empty().then_some("true")>
            <button
                class="c-datepicker__trigger c-btn"
                type="button"
                aria-haspopup="dialog"
                aria-expanded=move || open.get().to_string()
                aria-label=aria_label
                disabled=disabled
                node_ref=trigger_ref
                on:click=move |_| toggle()
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "ArrowDown" {
                        ev.prevent_default();
                        open_panel();
                    }
                }
            >
                <span class="c-datepicker__value">
                    {move || if current_key().is_empty() { blank.clone() } else { current_key() }}
                </span>
                <span class="c-datepicker__icon">
                    <Icon name=IconName::CALENDAR_RANGE size=14 />
                </span>
            </button>

            // 点外关闭不再用全屏遮罩：遮罩留在带 transform 的祖先里会被缩成
            // 祖先那么大，点外面关不掉。改由下面的文档级 pointerdown 判定。
            <Portal>
                {move || view! {
                    <div
                        class="c-datepicker__panel"
                        role="dialog"
                        aria-modal="false"
                        aria-label="选择日期"
                        tabindex="-1"
                        node_ref=panel_ref
                        style=move || if open.get() {
                            format!(
                                "z-index:{};position:fixed;left:-10000px;top:0px;visibility:hidden",
                                layer_z()
                            )
                        } else {
                            "display:none".to_string()
                        }
                    >
                <div class="c-datepicker__head">
                    <button
                        class="c-datepicker__nav"
                        type="button"
                        aria-label="上个月"
                        disabled=move || !can_shift(-1)
                        on:click=move |_| shift(-1)
                    >
                        <Icon name=IconName::CHEVRON_LEFT size=14 />
                    </button>
                    <div class="c-datepicker__title">{move || {
                        let (y, m) = view.get();
                        format!("{y} 年 {m} 月")
                    }}</div>
                    <button
                        class="c-datepicker__nav"
                        type="button"
                        aria-label="下个月"
                        disabled=move || !can_shift(1)
                        on:click=move |_| shift(1)
                    >
                        <Icon name=IconName::CHEVRON_RIGHT size=14 />
                    </button>
                </div>

                <table class="c-datepicker__grid" role="grid" aria-label=move || {
                    let (y, m) = view.get();
                    format!("{y} 年 {m} 月")
                }>
                    <thead>
                        <tr>
                            {WEEKDAYS
                                .iter()
                                .map(|w| view! {
                                    <th class="c-datepicker__weekday" scope="col">{*w}</th>
                                })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let (vy, vm) = view.get();
                            // 周一起始：先退到本月 1 号所在那一周的周一
                            let back = weekday_monday(vy, vm, 1) as i64;
                            let (sy, sm, sd) = add_days(vy, vm, 1, -back);
                            let sel_key = current_key();
                            let tkey = today_key.get_value();
                            // 固定 6 行：翻月时面板高度不跳，指针不用重新找落点
                            (0..6)
                                .map(|row| {
                                    view! {
                                        <tr>
                                            {(0..7)
                                                .map(|col| {
                                                    let (y, m, d) = add_days(sy, sm, sd, row * 7 + col);
                                                    let key = date_key(y, m, d);
                                                    let tab = if focus_day.get() == (y, m, d) { "0" } else { "-1" };
                                                    let ok = enabled(&key);
                                                    let selected = sel_key == key;
                                                    let is_today = tkey == key;
                                                    let outside = m != vm;
                                                    // 读屏念「2026 年 11 月 20 日」比念裸 key 有用
                                                    let label = format!("{y} 年 {m} 月 {d} 日");
                                                    view! {
                                                        <td>
                                                            <button
                                                                class="c-datepicker__day"
                                                                type="button"
                                                                tabindex=tab
                                                                data-day=key
                                                                data-outside=outside.then_some("true")
                                                                aria-label=label
                                                                aria-selected=selected.to_string()
                                                                aria-current=is_today.then_some("date")
                                                                aria-disabled=(!ok).then_some("true")
                                                                on:click=move |_| {
                                                                    if ok {
                                                                        pick(y, m, d);
                                                                    }
                                                                }
                                                                on:keydown=move |ev| on_day_key(ev, y, m, d)
                                                            >
                                                                {d}
                                                            </button>
                                                        </td>
                                                    }
                                                })
                                                .collect_view()}
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>

                {clearable.then(|| view! {
                    <div class="c-datepicker__foot">
                        <button
                            class="c-datepicker__clear"
                            type="button"
                            disabled=move || current_key().is_empty()
                            on:click=move |_| clear()
                        >
                            "清除"
                        </button>
                    </div>
                })}
                    </div>
                }}
            </Portal>
        </div>
    }
}

/* ==========================================================================
   时间 TimePicker
   ========================================================================== */

/// 时间选择：24 小时制两段、12 小时制三段，段内上下步进、数字直输。
///
/// 值是 `HH:mm`（24 小时制），12 小时制只是显示层。编辑中的值存成「一天内
/// 的分钟数」—— 加减与夹取都在整数上做，不构造 `Date`，夏令时那两天也不会
/// 凭空多出或少掉一小时。
///
/// 提交模型是「草稿」：上下键与数字只改草稿，Enter 才回 `on_change`，
/// Esc 把草稿丢掉、回到 `value`。
#[component]
pub fn TimePicker(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] hour12: bool,
    #[prop(into, optional)] label: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    let seg_refs = [
        NodeRef::<html::Span>::new(),
        NodeRef::<html::Span>::new(),
        NodeRef::<html::Span>::new(),
    ];
    // None = 跟着 value 走；Some(..) = 正在编辑的草稿
    let draft = RwSignal::new(None::<Option<u32>>);
    // 数字键的缓冲：(段, 已累积的值, 已按几位)。超过 800ms 不清空也不影响 ——
    // 换段或满两位就自动作废，省掉一个定时器。
    let buf = RwSignal::new((usize::MAX, 0u32, 0u32));
    let seg_count = if hour12 { 3 } else { 2 };

    let total = move || {
        draft
            .get()
            .unwrap_or_else(|| parse_hm(&value.get().unwrap_or_default()))
    };
    let hour_of = move |t: u32| t / 60;
    let minute_of = move |t: u32| t % 60;

    let focus_seg = move |i: usize| {
        if let Some(el) = seg_refs[i.min(2)].get() {
            let _ = el.focus();
        }
    };

    let bump = move |seg: usize, delta: i32| {
        let cur = total();
        let next = match seg {
            0 => {
                let t = cur.unwrap_or(0);
                let h = (hour_of(t) as i32 + delta).rem_euclid(24) as u32;
                Some(h * 60 + minute_of(t))
            }
            1 => {
                let t = cur.unwrap_or(0);
                let m = (minute_of(t) as i32 + delta).rem_euclid(60) as u32;
                Some(hour_of(t) * 60 + m)
            }
            // 上午/下午：整段挪 12 小时
            _ => {
                let t = cur.unwrap_or(0);
                Some(if hour_of(t) < 12 { t + 720 } else { t - 720 })
            }
        };
        draft.set(Some(next));
    };

    let type_digit = move |seg: usize, digit: u32| {
        let (bseg, bval, bcount) = buf.get_untracked();
        let accum = if bseg == seg && bcount > 0 {
            bval * 10 + digit
        } else {
            digit
        };
        let max = if seg == 0 { 23 } else { 59 };
        // 拼不出合法值（如时段的 2 → 5 = 25）就当成新的一位
        let val = if accum > max { digit } else { accum };
        let count = if bseg == seg && val == accum { bcount + 1 } else { 1 };
        let cur = total();
        let next = if seg == 0 {
            Some(val * 60 + cur.map(minute_of).unwrap_or(0))
        } else {
            Some(cur.map(hour_of).unwrap_or(0) * 60 + val)
        };
        draft.set(Some(next));
        // 还能再吃一位就等着；否则进位到下一段
        if count < 2 && val * 10 <= max {
            buf.set((seg, val, count));
        } else {
            buf.set((usize::MAX, 0, 0));
            if seg == 0 && seg_count > 1 {
                focus_seg(1);
            }
        }
    };

    let commit = move || {
        if let Some(t) = total() {
            let s = format!("{:02}:{:02}", hour_of(t), minute_of(t));
            if let Some(cb) = on_change {
                cb.run(s);
            }
        }
        // 回到「跟随 value」：父组件更新了就显示新值，没更新就还原
        draft.set(None);
    };

    let on_seg_key = move |ev: web_sys::KeyboardEvent, seg: usize| {
        if disabled || ev.is_composing() || ev.key_code() == 229 {
            return;
        }
        let key = ev.key();
        match key.as_str() {
            "ArrowLeft" | "ArrowRight" => {
                ev.prevent_default();
                let dir = if key == "ArrowRight" { 1 } else { -1 };
                let next = (seg as i32 + dir).clamp(0, seg_count as i32 - 1) as usize;
                focus_seg(next);
            }
            "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown" => {
                ev.prevent_default();
                let up = key == "ArrowUp" || key == "PageUp";
                let unit = if ev.shift_key() { 5 } else { 1 };
                bump(seg, if up { unit } else { -unit });
            }
            "Enter" => {
                ev.prevent_default();
                commit();
            }
            "Escape" => {
                // 不改外面的值，只丢草稿
                ev.prevent_default();
                draft.set(None);
            }
            _ => {
                let mut chars = key.chars();
                let Some(c) = chars.next() else { return };
                if chars.next().is_some() {
                    return;
                }
                if c.is_ascii_digit() && seg < 2 {
                    ev.prevent_default();
                    if let Some(d) = c.to_digit(10) {
                        type_digit(seg, d);
                    }
                } else if seg == 2 && matches!(c, 'a' | 'A' | 'p' | 'P') {
                    ev.prevent_default();
                    let want_pm = c.eq_ignore_ascii_case(&'p');
                    let is_pm = total().map(|t| hour_of(t) >= 12).unwrap_or(false);
                    if want_pm != is_pm {
                        bump(2, 1);
                    }
                }
            }
        }
    };

    let hh_text = move || match total() {
        None => "--".to_string(),
        Some(t) => {
            let h = hour_of(t);
            if hour12 {
                format!("{:02}", ((h + 11) % 12) + 1)
            } else {
                format!("{h:02}")
            }
        }
    };
    let mm_text = move || match total() {
        None => "--".to_string(),
        Some(t) => format!("{:02}", minute_of(t)),
    };
    let mer_text = move || {
        if total().map(|t| hour_of(t) >= 12).unwrap_or(false) {
            "下午"
        } else {
            "上午"
        }
    };
    let aria_label = if label.is_empty() {
        "时间".to_string()
    } else {
        label.clone()
    };

    view! {
        <div
            class="c-timepicker"
            role="group"
            aria-label=aria_label
            aria-disabled=disabled.then_some("true")
            data-empty=move || total().is_none().then_some("true")
        >
            <span
                class="c-timepicker__seg"
                role="spinbutton"
                tabindex=if disabled { "-1" } else { "0" }
                aria-label="小时"
                aria-valuemin="0"
                aria-valuemax=if hour12 { "12" } else { "23" }
                aria-valuenow=move || total().map(|t| {
                    let h = hour_of(t);
                    if hour12 { ((h + 11) % 12) + 1 } else { h }
                })
                node_ref=seg_refs[0]
                on:keydown=move |ev| on_seg_key(ev, 0)
            >
                {hh_text}
            </span>
            <span class="c-timepicker__colon" aria-hidden="true">":"</span>
            <span
                class="c-timepicker__seg"
                role="spinbutton"
                tabindex=if disabled { "-1" } else { "0" }
                aria-label="分钟"
                aria-valuemin="0"
                aria-valuemax="59"
                aria-valuenow=move || total().map(minute_of)
                node_ref=seg_refs[1]
                on:keydown=move |ev| on_seg_key(ev, 1)
            >
                {mm_text}
            </span>
            {hour12.then(|| view! {
                <span
                    class="c-timepicker__seg c-timepicker__meridiem"
                    role="spinbutton"
                    tabindex=if disabled { "-1" } else { "0" }
                    aria-label="上午或下午"
                    node_ref=seg_refs[2]
                    on:keydown=move |ev| on_seg_key(ev, 2)
                >
                    {mer_text}
                </span>
            })}
        </div>
    }
}
