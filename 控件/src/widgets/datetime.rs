use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::anchor::{self, AnchorSpec, AnchorWatch};
use crate::base::i18n::{fill, t};
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

/// 周表头不在这里写死：顺序（周一…周日）是网格的列序，**文案**随语言走
/// （`Strings::weekdays`），所以渲染时才查表。

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

/// 一天内的秒数。`HH:mm` 与 `HH:mm:ss` 都收 —— 后者是 `show_seconds` 打开后
/// 自己写出去的值，读不回来就会「选了秒、再打开秒没了」。
fn parse_time(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    let mut parts = raw.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let s: u32 = match parts.next() {
        Some(seg) => seg.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() || h >= 24 || m >= 60 || s >= 60 {
        return None;
    }
    Some(h * 3600 + m * 60 + s)
}

fn format_time(t: u32, show_seconds: bool) -> String {
    let (h, m, s) = (t / 3600, (t / 60) % 60, t % 60);
    if show_seconds {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}")
    }
}

/// 秒的步长只认 1..=60。0 与超界一律当 60（「只到分钟」）：
/// 否则网格里 `60 / step` 会除零，或者出现一个 0 格的秒列。
fn norm_step(step: u32) -> u32 {
    if step == 0 || step > 60 {
        60
    } else {
        step
    }
}

/// 把秒数对齐到步长。跨分钟由整除自动处理，不额外进位：
/// `step=15` 的 09:30:47 → 09:30:45，`step=60` 的 09:30:47 → 09:30:00。
fn snap_seconds(t: u32, step: u32) -> u32 {
    let s = t % 60;
    (t - s) + (s / step) * step
}

/// 区间两端排序：第二次点的日子早于第一次就交换。
///
/// 抽成纯函数是为了能单测 —— 两次点击的先后由用户决定，`(起, 止)` 不能出现逆序，
/// 而这段逻辑埋在浮层闭包里时没有任何自动手段能验证它。
fn order_pair(a: (i32, u32, u32), b: (i32, u32, u32)) -> ((i32, u32, u32), (i32, u32, u32)) {
    if days_from_civil(a.0, a.1, a.2) <= days_from_civil(b.0, b.1, b.2) {
        (a, b)
    } else {
        (b, a)
    }
}

/// 时间区间跨零点：两端都有值，且起点晚于终点。**这不是错误**，只是需要提示，
/// 所以没有夹取、没有交换 —— 23:30 → 01:00 是合法的两小时。
fn crosses_midnight(a: &str, b: &str) -> bool {
    match (parse_time(a), parse_time(b)) {
        (Some(x), Some(y)) => x > y,
        _ => false,
    }
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

/// 网格里 PageUp/PageDown 一次跳几格。与 CSS 的列高（约 6 行）对齐 ——
/// 翻一屏就该是「下一页从上一页的最后一行下面开始」，跳 1 格那是 ↑↓ 的活。
const GRID_PAGE: usize = 6;

/// 把刚聚焦的格子滚进**它所在那一列**的可见区。
///
/// 网格是三列各自滚动的列表：滚整块面板会把用户刚调好的另一列滚走。
/// 用 `scrollTop` 做最小位移而不是 `scrollIntoView`，后者还会顺手滚动整个页面。
fn reveal_in_column(cell: &web_sys::HtmlElement) {
    let Some(parent) = cell.parent_element() else {
        return;
    };
    let Ok(col) = parent.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };
    let cr = col.get_bounding_client_rect();
    let er = cell.get_bounding_client_rect();
    if er.top() < cr.top() {
        col.set_scroll_top(col.scroll_top() - (cr.top() - er.top()).round() as i32);
    } else if er.bottom() > cr.bottom() {
        col.set_scroll_top(col.scroll_top() + (er.bottom() - cr.bottom()).round() as i32);
    }
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
        // `push_adopted`：在气泡 / 流程面板里时挂到容器那层下面，否则等价 `push`。
        let id = host.push_adopted(OverlayKind::Popover, Callback::new(move |_| close()));
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
            // 把面板交给栈：外层气泡的「点外关闭」要认出「这一下点的是我的子面板」。
            if let Some(id) = layer.get_untracked() {
                host.bind(id, panel.clone().into());
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

    // 兜底值一律查文案表，且留在属性/文本闭包里 —— 切语言时这两处要跟着变。
    let label_store = StoredValue::new_local(label);
    let blank_store = StoredValue::new_local(placeholder);
    let aria_label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().date_label.to_string()
        } else {
            own
        }
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
                    {move || {
                        if !current_key().is_empty() {
                            return current_key();
                        }
                        let own = blank_store.get_value();
                        if own.is_empty() {
                            t().pick_date.to_string()
                        } else {
                            own
                        }
                    }}
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
                        aria-label=move || t().pick_date.to_string()
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
                        aria-label=move || t().prev_month.to_string()
                        disabled=move || !can_shift(-1)
                        on:click=move |_| shift(-1)
                    >
                        <Icon name=IconName::CHEVRON_LEFT size=14 />
                    </button>
                    <div class="c-datepicker__title">{move || {
                        let (y, m) = view.get();
                        fill(t().month_year_fmt, &[("y", &y.to_string()), ("m", &m.to_string())])
                    }}</div>
                    <button
                        class="c-datepicker__nav"
                        type="button"
                        aria-label=move || t().next_month.to_string()
                        disabled=move || !can_shift(1)
                        on:click=move |_| shift(1)
                    >
                        <Icon name=IconName::CHEVRON_RIGHT size=14 />
                    </button>
                </div>

                <table class="c-datepicker__grid" role="grid" aria-label=move || {
                    let (y, m) = view.get();
                    fill(t().month_year_fmt, &[("y", &y.to_string()), ("m", &m.to_string())])
                }>
                    <thead>
                        <tr>
                            {move || {
                                t().weekdays
                                    .iter()
                                    .map(|w| view! {
                                        <th class="c-datepicker__weekday" scope="col">{*w}</th>
                                    })
                                    .collect_view()
                            }}
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
                                                    let label = fill(
                                                        t().day_label_fmt,
                                                        &[
                                                            ("y", &y.to_string()),
                                                            ("m", &m.to_string()),
                                                            ("d", &d.to_string()),
                                                        ],
                                                    );
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
                            {move || t().clear}
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
   日期区间 DateRange
   ========================================================================== */

/// 日期区间：单月面板，两次点击定「起 → 止」。值是 `(YYYY-MM-DD, YYYY-MM-DD)`，
/// 与 `DatePicker` 同一套字符串约定（定宽，字符串比较即日期比较）。
///
/// 三件容易被做错的事，这里定死：
/// 1. **顺序**：第二次点的日子早于第一次就交换，永远不产出逆序区间；
/// 2. **不死锁**：已经有完整区间时再点任意一天，就从那天重新开始（旧区间只停止
///    高亮，新值在第二次点击时才上报），不会出现「点哪都没反应」；
/// 3. **预览**：终点未定时，悬停到哪天就把起点到那天涂上 `data-preview`；
///    键盘移动焦点走同一条预览路径，键盘用户点之前也看得见会选中什么。
///
/// 面板是 `Portal` + `anchor` 定位（红线 4）；Esc 只交给 `OverlayHost`，
/// 组件自己不挂 `document` 监听。
#[component]
pub fn DateRange(
    #[prop(into, optional)] value: MaybeProp<(String, String)>,
    #[prop(into, optional)] min: String,
    #[prop(into, optional)] max: String,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] placeholder: String,
    /// 非空时作为触发器的内联宽度（如 `"280px"`）；空则跟 `DatePicker` 同宽
    #[prop(into, optional)] width: String,
    /// 面板底部是否给「清除」
    #[prop(default = true)] clearable: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<(String, String)>>,
) -> impl IntoView {
    let host = OverlayHost::use_or_provide();
    let trigger_ref = NodeRef::<html::Button>::new();
    let panel_ref = NodeRef::<html::Div>::new();

    let open = RwSignal::new(false);
    let layer = RwSignal::new(None::<OverlayId>);
    let watch = StoredValue::new_local(None::<AnchorWatch>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
    let view = RwSignal::new({
        let t = today();
        (t.0, t.1)
    });
    let focus_day = RwSignal::new(today());
    // 第一次点击定下的起点；`None` = 还在等第一次点击
    let pending = RwSignal::new(None::<(i32, u32, u32)>);
    // 指针停在哪天（键盘移动时清空，让预览跟着焦点走）
    let hover = RwSignal::new(None::<(i32, u32, u32)>);

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

    // 外部值统一成定宽 key：`2026-1-5` 这种写法也认得，比较和显示都用归一化后的
    let pair = move || -> (String, String) {
        let (a, b) = value.get().unwrap_or_default();
        (
            parse_date(&a)
                .map(|(y, m, d)| date_key(y, m, d))
                .unwrap_or_default(),
            parse_date(&b)
                .map(|(y, m, d)| date_key(y, m, d))
                .unwrap_or_default(),
        )
    };
    let start_key = move || pair().0;
    let end_key = move || pair().1;

    // 已提交的区间（两端都合法才算）。排序交给 order_pair，调用方写反也不会画歪
    let committed = move || -> Option<((i32, u32, u32), (i32, u32, u32))> {
        match (parse_date(&start_key()), parse_date(&end_key())) {
            (Some(a), Some(b)) => Some(order_pair(a, b)),
            _ => None,
        }
    };

    // 三种格子外观各自独立求值：属性级响应式，不重建 DOM ——
    // 整块重建会把键盘焦点连同节点一起丢掉，鼠标一划过就把焦点赶走。
    let is_selected = move |y: i32, m: u32, d: u32| -> bool {
        match pending.get() {
            // 正在重选：旧区间的端点不再算「已选」，否则用户看不出自己开始了新选择
            Some(p) => p == (y, m, d),
            None => committed()
                .map(|(a, b)| (y, m, d) == a || (y, m, d) == b)
                .unwrap_or(false),
        }
    };
    let in_range = move |y: i32, m: u32, d: u32| -> bool {
        if pending.get().is_some() {
            return false;
        }
        match committed() {
            Some((a, b)) => {
                let di = days_from_civil(y, m, d);
                di > days_from_civil(a.0, a.1, a.2) && di < days_from_civil(b.0, b.1, b.2)
            }
            None => false,
        }
    };
    let previewing = move |y: i32, m: u32, d: u32| -> bool {
        let Some(p) = pending.get() else { return false };
        if p == (y, m, d) {
            return false;
        }
        // 指针在哪天算哪天；指针没进过网格（纯键盘）就跟着焦点走
        let Some(h) = hover.get().or_else(|| Some(focus_day.get())) else {
            return false;
        };
        let anchor = days_from_civil(p.0, p.1, p.2);
        let head = days_from_civil(h.0, h.1, h.2);
        let di = days_from_civil(y, m, d);
        di >= anchor.min(head) && di <= anchor.max(head)
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

    // 焦点是否还在网格里：翻月按钮被鼠标点过之后不该把焦点抢回格子
    let in_grid = move || {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .map(|el| el.class_list().contains("c-datepicker__day"))
            .unwrap_or(false)
    };

    let close = move || {
        open.set(false);
        // 收起时把「等第二个点」的状态也清掉：下次打开是一次全新的选择，
        // 否则用户会看到上次留下的半个区间
        pending.set(None);
        hover.set(None);
        watch.set_value(None);
        outside.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    };

    let emit = move |a: (i32, u32, u32), b: (i32, u32, u32)| {
        let (x, y) = order_pair(a, b);
        if let Some(cb) = on_change {
            cb.run((
                date_key(x.0, x.1, x.2),
                date_key(y.0, y.1, y.2),
            ));
        }
    };

    // 点击 = 键盘 Enter：两条入口必须完全同义，否则鼠标能选、键盘不能
    let press = move |y: i32, m: u32, d: u32| {
        let key = date_key(y, m, d);
        if !enabled(&key) {
            return;
        }
        match pending.get_untracked() {
            None => {
                pending.set(Some((y, m, d)));
                hover.set(None);
            }
            Some(first) => {
                pending.set(None);
                hover.set(None);
                close();
                emit(first, (y, m, d));
            }
        }
    };

    let clear = move || {
        close();
        if let Some(cb) = on_change {
            cb.run((String::new(), String::new()));
        }
    };

    let open_panel = move || {
        if disabled || layer.get_untracked().is_some() {
            return;
        }
        // 打开时落在已选起点（只有终点时落在终点），都没有就落今天
        let seed = committed()
            .map(|(a, _)| a)
            .or_else(|| parse_date(&start_key()))
            .or_else(|| parse_date(&end_key()))
            .unwrap_or_else(today);
        let seed = clamp_day(seed.0, seed.1, seed.2);
        view.set((seed.0, seed.1));
        focus_day.set(seed);
        pending.set(None);
        hover.set(None);
        // `push_adopted`：在气泡 / 流程面板里时挂到容器那层下面，否则等价 `push`。
        let id = host.push_adopted(OverlayKind::Popover, Callback::new(move |_| close()));
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

    let shift = move |dir: i32| {
        let (fy, fm, fd) = focus_day.get_untracked();
        let (ty, tm, td) = add_months(fy, fm, fd, dir);
        let (ty, tm, td) = clamp_day(ty, tm, td);
        focus_day.set((ty, tm, td));
        view.set((ty, tm));
        hover.set(None);
        if in_grid() {
            restore(date_key(ty, tm, td));
        }
    };

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
            // 键盘移动就把指针的悬停作废，预览跟着焦点走
            hover.set(None);
            restore(date_key(ty, tm, td));
            return;
        }
        if ev.key() == "Enter" || ev.key() == " " {
            ev.prevent_default();
            press(y, m, d);
        }
    };

    let layer_z = move || {
        layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Popover.base_z())
    };

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
            return;
        };

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
            if !open.get_untracked() {
                return;
            }
            if !anchor_el.is_connected() || !panel.is_connected() {
                return;
            }
            // 把面板交给栈：外层气泡的「点外关闭」要认出「这一下点的是我的子面板」。
            if let Some(id) = layer.get_untracked() {
                host.bind(id, panel.clone().into());
            }
            let spec = AnchorSpec {
                match_width: false,
                min_width: 236.0,
                scroll: false,
                ..AnchorSpec::bubble()
            };
            anchor::place_and_apply(&anchor_el, &panel, spec);
            let (y, m, d) = focus_day.get_untracked();
            restore(date_key(y, m, d));
            let on_gone: Rc<dyn Fn()> = Rc::new(move || close());
            watch.set_value(Some(anchor::watch(anchor_el, panel.into(), spec, Some(on_gone))));
        });
    });

    // 与 DatePicker 同一套：兜底查表，留在闭包里。
    let label_store = StoredValue::new_local(label);
    let blank_store = StoredValue::new_local(placeholder);
    let aria_label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().date_range_label.to_string()
        } else {
            own
        }
    };
    let width_style = if width.trim().is_empty() {
        String::new()
    } else {
        format!("width:{}", width.trim())
    };

    on_cleanup(move || {
        watch.set_value(None);
        outside.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    view! {
        <div
            class="c-datepicker c-daterange"
            data-empty=move || {
                let (a, b) = pair();
                (a.is_empty() && b.is_empty()).then_some("true")
            }
        >
            <button
                class="c-datepicker__trigger c-btn"
                type="button"
                style=width_style
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
                    {move || {
                        let (a, b) = pair();
                        match (a.is_empty(), b.is_empty()) {
                            (true, true) => {
                                let own = blank_store.get_value();
                                if own.is_empty() {
                                    t().pick_range_blank.to_string()
                                } else {
                                    own
                                }
                            }
                            (false, true) => format!("{a} → …"),
                            (true, false) => format!("… → {b}"),
                            _ => format!("{a} → {b}"),
                        }
                    }}
                </span>
                <span class="c-datepicker__icon">
                    <Icon name=IconName::CALENDAR_RANGE size=14 />
                </span>
            </button>

            <Portal>
                {move || view! {
                    <div
                        class="c-datepicker__panel c-daterange__panel"
                        role="dialog"
                        aria-modal="false"
                        aria-label=move || t().pick_date_range.to_string()
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
                                aria-label=move || t().prev_month.to_string()
                                disabled=move || !can_shift(-1)
                                on:click=move |_| shift(-1)
                            >
                                <Icon name=IconName::CHEVRON_LEFT size=14 />
                            </button>
                            <div class="c-datepicker__title">{move || {
                                let (y, m) = view.get();
                                fill(t().month_year_fmt, &[("y", &y.to_string()), ("m", &m.to_string())])
                            }}</div>
                            <button
                                class="c-datepicker__nav"
                                type="button"
                                aria-label=move || t().next_month.to_string()
                                disabled=move || !can_shift(1)
                                on:click=move |_| shift(1)
                            >
                                <Icon name=IconName::CHEVRON_RIGHT size=14 />
                            </button>
                        </div>

                        <table
                            class="c-datepicker__grid"
                            role="grid"
                            aria-label=move || {
                                let (y, m) = view.get();
                                fill(t().month_year_fmt, &[("y", &y.to_string()), ("m", &m.to_string())])
                            }
                            // 指针离开网格就撤掉预览：预览是「会选到哪」，鼠标已经不在了
                            // 还留着一条带子会让人以为那是已选区间
                            on:pointerleave=move |_| hover.set(None)
                        >
                            <thead>
                                <tr>
                                    {move || {
                                        t().weekdays
                                            .iter()
                                            .map(|w| view! {
                                                <th class="c-datepicker__weekday" scope="col">{*w}</th>
                                            })
                                            .collect_view()
                                    }}
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    // 这里只读 view：换月才重建 42 格（必须重建，日号变了）。
                                    // 选中 / 区间 / 预览都挂在每格的响应式属性上。
                                    let (vy, vm) = view.get();
                                    let back = weekday_monday(vy, vm, 1) as i64;
                                    let (sy, sm, sd) = add_days(vy, vm, 1, -back);
                                    let tkey = today_key.get_value();
                                    (0..6)
                                        .map(|row| {
                                            view! {
                                                <tr>
                                                    {(0..7)
                                                        .map(|col| {
                                                            let (y, m, d) = add_days(sy, sm, sd, row * 7 + col);
                                                            let key = date_key(y, m, d);
                                                            let ok = enabled(&key);
                                                            let is_today = tkey == key;
                                                            let outside = m != vm;
                                                            let label = fill(
                                                                t().day_label_fmt,
                                                                &[
                                                                    ("y", &y.to_string()),
                                                                    ("m", &m.to_string()),
                                                                    ("d", &d.to_string()),
                                                                ],
                                                            );
                                                            view! {
                                                                <td
                                                                    role="gridcell"
                                                                    aria-selected=move || is_selected(y, m, d).to_string()
                                                                >
                                                                    <button
                                                                        class="c-datepicker__day"
                                                                        type="button"
                                                                        tabindex=move || if focus_day.get() == (y, m, d) { "0" } else { "-1" }
                                                                        data-day=key
                                                                        data-outside=outside.then_some("true")
                                                                        data-in-range=move || in_range(y, m, d).then_some("true")
                                                                        data-preview=move || previewing(y, m, d).then_some("true")
                                                                        aria-label=label
                                                                        aria-current=is_today.then_some("date")
                                                                        aria-disabled=(!ok).then_some("true")
                                                                        on:pointerenter=move |_| hover.set(Some((y, m, d)))
                                                                        on:click=move |_| press(y, m, d)
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
                                    disabled=move || {
                                        let (a, b) = pair();
                                        a.is_empty() && b.is_empty()
                                    }
                                    on:click=move |_| clear()
                                >
                                    {move || t().clear}
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

/// 时间选择。值是 `HH:mm`；`show_seconds` 打开时是 `HH:mm:ss`。
///
/// 两种形态，共用同一套取值模型（一天内的秒数，加减全在整数上做，不构造 `Date`，
/// 夏令时那两天也不会凭空多出或少掉一小时）：
///
/// - **段式（默认）**：24 小时制两段、12 小时制三段（加秒则各多一段），
///   段内 ↑↓ 步进、数字直输、←→ 换段。提交是「草稿」模型：↑↓ 与数字只改草稿，
///   Enter 才回 `on_change`，Esc 丢掉草稿回到 `value`。
/// - **网格（`grid = true`）**：时 / 分 / 秒三列按钮，每列自己滚。方向键列内移动、
///   ←→ 换列、Home/End 到列首末、PageUp/PageDown 翻一屏。网格里点一格就是选定
///   一个分量，随即上报 —— 它没有「草稿」可言。网格固定 24 小时制（列格就是 0..23），
///   `hour12` 只作用在段式。
///
/// `step_secs` 只约束秒：15 / 30 时秒段与秒列按步长取值，60 表示「只到分钟」
/// （提交前把秒对到整分）。
#[component]
pub fn TimePicker(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] hour12: bool,
    #[prop(into, optional)] label: String,
    #[prop(optional)] disabled: bool,
    /// 秒的步进（1..=60，常用 15 / 30 / 60）。60 = 只到分钟
    #[prop(default = 60)] step_secs: u32,
    /// 值写成 `HH:mm:ss`，并显示秒段 / 秒列
    #[prop(optional)] show_seconds: bool,
    /// 网格布局：三列按钮并排，而不是数字段
    #[prop(optional)] grid: bool,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    let step = norm_step(step_secs);
    let root_ref = NodeRef::<html::Div>::new();
    let seg_refs = [
        NodeRef::<html::Span>::new(),
        NodeRef::<html::Span>::new(),
        NodeRef::<html::Span>::new(),
        NodeRef::<html::Span>::new(),
    ];
    // None = 跟着 value 走；Some(..) = 正在编辑的草稿
    let draft = RwSignal::new(None::<Option<u32>>);
    // 数字键的缓冲：(段, 已累积的值, 已按几位)。换段或满两位就自动作废，
    // 省掉一个定时器。
    let buf = RwSignal::new((usize::MAX, 0u32, 0u32));

    // 秒段是否存在决定后面所有下标：不能写死「第 2 段就是上午/下午」。
    let sec_idx = 2usize;
    let mer_idx = if show_seconds { 3 } else { 2 };
    let seg_count = mer_idx + usize::from(hour12);

    // 网格的列数（时/分/秒）与秒列格数。步长 60 时秒列只有「00」一格 ——
    // 那正是在说「秒固定为 0」，不需要为它特判一个隐藏分支。
    let col_count = 2 + usize::from(show_seconds);
    let sec_cells = (60 / step).max(1) as usize;
    // 网格的光标：Tab 进来先落在当前值那一格上
    let seed_idx = parse_time(&value.get().unwrap_or_default()).unwrap_or(0) / 3600;
    let grid_focus = RwSignal::new((0usize, seed_idx as usize));

    let total = move || {
        draft
            .get()
            .unwrap_or_else(|| parse_time(&value.get().unwrap_or_default()))
    };
    let hour_of = move |t: u32| t / 3600;
    let minute_of = move |t: u32| (t / 60) % 60;
    let second_of = move |t: u32| t % 60;

    let focus_seg = move |i: usize| {
        if let Some(el) = seg_refs[i.min(3)].get() {
            let _ = el.focus();
        }
    };

    // 一次增减就是「加多少秒」：时 3600、分 60、秒 step、上午下午 43200。
    // 全部对 86400 取模，跨零点/跨中午都自然成立。
    let bump = move |seg: usize, delta: i32| {
        let cur = total().unwrap_or(0) as i64;
        let unit = if seg == 0 {
            3600
        } else if seg == 1 {
            60
        } else if show_seconds && seg == sec_idx {
            step as i64
        } else {
            43_200
        };
        let next = (cur + delta as i64 * unit).rem_euclid(86_400) as u32;
        draft.set(Some(Some(next)));
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
        let cur = total().unwrap_or(0);
        let next = match seg {
            0 => val * 3600 + cur % 3600,
            1 => (cur / 3600) * 3600 + val * 60 + cur % 60,
            _ => (cur / 60) * 60 + val,
        };
        draft.set(Some(Some(next)));
        // 还能再吃一位就等着；否则进位到下一段。上午/下午不是数字段，不参与进位 ——
        // 这样 12 小时制下「时分钟打完」的落点和以前完全一致。
        if count < 2 && val * 10 <= max {
            buf.set((seg, val, count));
        } else {
            buf.set((usize::MAX, 0, 0));
            if seg + 1 < seg_count && seg + 1 != mer_idx {
                focus_seg(seg + 1);
            }
        }
    };

    let emit = move |t: u32| {
        if let Some(cb) = on_change {
            cb.run(format_time(t, show_seconds));
        }
    };

    let commit = move || {
        if let Some(t) = total() {
            emit(snap_seconds(t, step));
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
                let numeric = seg < 2 || (show_seconds && seg == sec_idx);
                if numeric && c.is_ascii_digit() {
                    ev.prevent_default();
                    if let Some(d) = c.to_digit(10) {
                        type_digit(seg, d);
                    }
                } else if seg == mer_idx && matches!(c, 'a' | 'A' | 'p' | 'P') {
                    ev.prevent_default();
                    let want_pm = c.eq_ignore_ascii_case(&'p');
                    let is_pm = total().map(|t| hour_of(t) >= 12).unwrap_or(false);
                    if want_pm != is_pm {
                        bump(mer_idx, 1);
                    }
                }
            }
        }
    };

    // —— 网格 ——

    let part_of = move |t: u32, col: usize| -> u32 {
        match col {
            0 => t / 3600,
            1 => (t / 60) % 60,
            _ => t % 60,
        }
    };
    let cell_value = move |col: usize, idx: usize| -> u32 {
        if col == 2 {
            idx as u32 * step
        } else {
            idx as u32
        }
    };
    let col_len = move |col: usize| -> usize {
        match col {
            0 => 24,
            1 => 60,
            _ => sec_cells,
        }
    };
    let with_part = move |col: usize, val: u32| -> u32 {
        let cur = total().unwrap_or(0);
        match col {
            0 => val * 3600 + cur % 3600,
            1 => (cur / 3600) * 3600 + val * 60 + cur % 60,
            _ => (cur / 60) * 60 + val,
        }
    };
    // 网格里每一格都是一个已确定的分量，点/Enter 就提交 —— 没有「半成品」状态
    let choose = move |col: usize, val: u32| {
        emit(snap_seconds(with_part(col, val), step));
        draft.set(None);
    };

    let focus_cell = move |col: usize, idx: usize| {
        schedule(move || {
            let Some(root) = root_ref.get() else { return };
            let sel = format!(".c-timepicker__cell[data-col=\"{col}\"][data-idx=\"{idx}\"]");
            if let Ok(Some(el)) = root.query_selector(&sel) {
                if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.focus();
                    reveal_in_column(&el);
                }
            }
        });
    };

    let on_cell_key = move |ev: web_sys::KeyboardEvent, col: usize, idx: usize| {
        if disabled {
            return;
        }
        let mut next = (col, idx);
        match ev.key().as_str() {
            "ArrowDown" => next.1 = (idx + 1).min(col_len(col) - 1),
            "ArrowUp" => next.1 = idx.saturating_sub(1),
            "ArrowRight" => {
                let c = (col + 1).min(col_count - 1);
                next = (c, idx.min(col_len(c) - 1));
            }
            "ArrowLeft" => {
                let c = col.saturating_sub(1);
                next = (c, idx.min(col_len(c) - 1));
            }
            "Home" => next.1 = 0,
            "End" => next.1 = col_len(col) - 1,
            "PageUp" => next.1 = idx.saturating_sub(GRID_PAGE),
            "PageDown" => next.1 = (idx + GRID_PAGE).min(col_len(col) - 1),
            "Enter" | " " => {
                ev.prevent_default();
                choose(col, cell_value(col, idx));
                return;
            }
            _ => return,
        }
        ev.prevent_default();
        grid_focus.set(next);
        focus_cell(next.0, next.1);
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
    let ss_text = move || match total() {
        None => "--".to_string(),
        Some(t) => format!("{:02}", second_of(t)),
    };
    let mer_text = move || {
        if total().map(|t| hour_of(t) >= 12).unwrap_or(false) {
            t().pm
        } else {
            t().am
        }
    };
    // 兜底查文案表，留在属性闭包里。
    let label_store = StoredValue::new_local(label);
    let aria_label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().time_label.to_string()
        } else {
            own
        }
    };

    // 段式的三段 + 可选秒段 + 可选上午下午
    let segments = view! {
        <span
            class="c-timepicker__seg"
            role="spinbutton"
            tabindex=if disabled { "-1" } else { "0" }
            aria-label=move || t().hour.to_string()
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
            aria-label=move || t().minute.to_string()
            aria-valuemin="0"
            aria-valuemax="59"
            aria-valuenow=move || total().map(minute_of)
            node_ref=seg_refs[1]
            on:keydown=move |ev| on_seg_key(ev, 1)
        >
            {mm_text}
        </span>
        {show_seconds.then(|| view! {
            <span class="c-timepicker__colon" aria-hidden="true">":"</span>
            <span
                class="c-timepicker__seg"
                role="spinbutton"
                tabindex=if disabled { "-1" } else { "0" }
                aria-label=move || t().second.to_string()
                aria-valuemin="0"
                aria-valuemax="59"
                aria-valuenow=move || total().map(second_of)
                node_ref=seg_refs[sec_idx]
                on:keydown=move |ev| on_seg_key(ev, sec_idx)
            >
                {ss_text}
            </span>
        })}
        {hour12.then(|| view! {
            <span
                class="c-timepicker__seg c-timepicker__meridiem"
                role="spinbutton"
                tabindex=if disabled { "-1" } else { "0" }
                aria-label=move || t().meridiem.to_string()
                node_ref=seg_refs[mer_idx]
                on:keydown=move |ev| on_seg_key(ev, mer_idx)
            >
                {mer_text}
            </span>
        })}
    }
    .into_any();

    // 网格：每列一个 listbox，格是 option，roving tabindex 只有一个 0。
    // 格子本身只渲染一次，选中态与 tabindex 都是**响应式属性** —— 整块重建会把
    // 焦点连同 DOM 一起丢掉，方向键就会在第一次移动后失联。
    let cells = view! {
        {(0..col_count)
            .map(|col| {
                let len = col_len(col);
                // 列名查表：这是 listbox 的可访问名，切语言要跟着变。
                let name = move || match col {
                    0 => t().hour.to_string(),
                    1 => t().minute.to_string(),
                    _ => t().second.to_string(),
                };
                view! {
                    <div class="c-timepicker__col" role="listbox" aria-label=name>
                        {(0..len)
                            .map(|i| {
                                let val = cell_value(col, i);
                                view! {
                                    <button
                                        class="c-timepicker__cell"
                                        type="button"
                                        role="option"
                                        tabindex=move || {
                                            // 禁用时不留可 Tab 的格子：与段式的 tabindex=-1 同义
                                            if disabled {
                                                return "-1";
                                            }
                                            let (fc, fi) = grid_focus.get();
                                            let fc = fc.min(col_count - 1);
                                            if fc == col && fi.min(len - 1) == i { "0" } else { "-1" }
                                        }
                                        aria-selected=move || {
                                            total()
                                                .map(|t| part_of(t, col) == val)
                                                .unwrap_or(false)
                                                .to_string()
                                        }
                                        data-col=col as i32
                                        data-idx=i as i32
                                        on:click=move |_| choose(col, val)
                                        on:keydown=move |ev| on_cell_key(ev, col, i)
                                    >
                                        {format!("{val:02}")}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                }
            })
            .collect_view()}
    }
    .into_any();

    view! {
        <div
            class=if grid { "c-timepicker c-timepicker--grid" } else { "c-timepicker" }
            role="group"
            aria-label=aria_label
            aria-disabled=disabled.then_some("true")
            data-empty=move || total().is_none().then_some("true")
            data-seconds=show_seconds.then_some("1")
            node_ref=root_ref
        >
            {if grid { cells } else { segments }}
        </div>
    }
}

/* ==========================================================================
   时间区间 TimeRange
   ========================================================================== */

/// 时间区间：两段时间并排，`on_change` 回 `(起, 止)`。
///
/// **跨零点是合法的**：23:30 → 01:00 表示两小时，不夹取、不交换、不报错，
/// 只在旁边标一句「跨零点」。要不要「必须 end > start」是业务判断，控件不该替
/// 调用方否决它。
///
/// 两个端点复用 `TimePicker`：段式的键盘、数字直输、草稿规则只维护一处；
/// 区间做的事就是「把单端变更拼成整对再上报」，另一端保持调用方给的值。
#[component]
pub fn TimeRange(
    #[prop(into, optional)] start: MaybeProp<String>,
    #[prop(into, optional)] end: MaybeProp<String>,
    #[prop(into, optional)] label: String,
    /// 秒的步进，透传给两个端点（1..=60，常用 15 / 30 / 60）
    #[prop(default = 60)] step_secs: u32,
    #[prop(optional)] show_seconds: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<(String, String)>>,
) -> impl IntoView {
    let start_now = move || start.get().unwrap_or_default();
    let end_now = move || end.get().unwrap_or_default();
    let crosses = move || crosses_midnight(&start_now(), &end_now());

    let on_start = move |v: String| {
        if let Some(cb) = on_change {
            cb.run((v, end_now()));
        }
    };
    let on_end = move |v: String| {
        if let Some(cb) = on_change {
            cb.run((start_now(), v));
        }
    };

    // 兜底查文案表，留在属性闭包里。
    let label_store = StoredValue::new_local(label);
    let aria_label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().time_range_label.to_string()
        } else {
            own
        }
    };

    view! {
        <div
            class="c-timerange"
            role="group"
            aria-label=aria_label
            data-crosses=move || crosses().then_some("true")
        >
            <TimePicker
                value=start
                label=t().start_time
                step_secs=step_secs
                show_seconds=show_seconds
                disabled=disabled
                on_change=Callback::new(on_start)
            />
            <span class="c-timerange__sep" aria-hidden="true">"–"</span>
            <TimePicker
                value=end
                label=t().end_time
                step_secs=step_secs
                show_seconds=show_seconds
                disabled=disabled
                on_change=Callback::new(on_end)
            />
            {move || crosses().then(|| view! {
                <span class="c-timerange__note">{move || t().crosses_midnight}</span>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_roundtrip_and_leap_day() {
        // 闰年 2/29 必须既算得出、也认得回来
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert!(parse_date("2024-02-29").is_some());
        assert!(parse_date("2026-02-29").is_none());
        // 定宽约定：非定宽写法一律不认，否则「字符串比较即日期比较」就不成立
        assert!(parse_date("2026-1-5").is_none());
        assert!(parse_date("2026-13-01").is_none());
        let (y, m, d) = civil_from_days(days_from_civil(2024, 2, 29));
        assert_eq!((y, m, d), (2024, 2, 29));
    }

    #[test]
    fn date_walks_across_month_and_year() {
        assert_eq!(add_days(2026, 12, 31, 1), (2027, 1, 1));
        assert_eq!(add_days(2024, 3, 1, -1), (2024, 2, 29));
        // 跨月时「日」要被夹到目标月长度，不能溢出成 3/3
        assert_eq!(add_months(2026, 1, 31, 1), (2026, 2, 28));
        assert_eq!(add_months(2024, 1, 31, 1), (2024, 2, 29));
        assert_eq!(add_months(2026, 3, 31, -1), (2026, 2, 28));
        assert_eq!(add_months(2026, 12, 15, 1), (2027, 1, 15));
    }

    #[test]
    fn range_pair_is_always_ordered() {
        // 反着点：先 11/27 再 11/20，必须交换
        assert_eq!(
            order_pair((2026, 11, 27), (2026, 11, 20)),
            ((2026, 11, 20), (2026, 11, 27))
        );
        // 起 = 止是合法区间，不能被判成非法
        assert_eq!(
            order_pair((2026, 11, 20), (2026, 11, 20)),
            ((2026, 11, 20), (2026, 11, 20))
        );
        // 跨年比较
        assert_eq!(
            order_pair((2027, 1, 1), (2026, 12, 31)),
            ((2026, 12, 31), (2027, 1, 1))
        );
    }

    #[test]
    fn time_parses_and_crosses_midnight() {
        assert_eq!(parse_time("09:30"), Some(9 * 3600 + 30 * 60));
        assert_eq!(parse_time("09:30:15"), Some(9 * 3600 + 30 * 60 + 15));
        assert_eq!(parse_time("24:00"), None);
        assert_eq!(parse_time("09:60"), None);
        assert_eq!(parse_time("09:30:60"), None);
        // 跨零点：不交换、不报错，但能被识别出来提示
        assert!(crosses_midnight("23:30", "01:00"));
        assert!(!crosses_midnight("01:00", "23:30"));
        // 缺一端时不该说「跨零点」
        assert!(!crosses_midnight("23:30", ""));
        // 23:30 → 01:00 的时长是 90 分钟，不是 -1350
        let a = parse_time("23:30").unwrap();
        let b = parse_time("01:00").unwrap();
        assert_eq!((b + 86_400 - a) / 60, 90);
    }

    #[test]
    fn seconds_snap_to_step() {
        assert_eq!(snap_seconds(9 * 3600 + 47, 15), 9 * 3600 + 45);
        assert_eq!(snap_seconds(9 * 3600 + 47, 60), 9 * 3600);
        assert_eq!(norm_step(0), 60);
        assert_eq!(norm_step(15), 15);
        assert_eq!(norm_step(120), 60);
        assert_eq!(format_time(9 * 3600 + 30 * 60 + 5, true), "09:30:05");
        assert_eq!(format_time(9 * 3600 + 30 * 60 + 5, false), "09:30");
    }
}
