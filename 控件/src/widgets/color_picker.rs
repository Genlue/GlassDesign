use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::base::icon::{Icon, IconName};

/* ==========================================================================
   颜色模型（hex ↔ rgb ↔ hsv）

   不加依赖：转换一共三十行，引入 `palette` 这种库反而要自己处理色空间与
   版本漂移。这里只做 sRGB，`#rrggbbaa` 的 alpha 用 0..1 的 f64 存。

   为什么内部存 HSV 而不是 RGB：SV 面板拖动时色相必须**保持不动**。若每次都
   从 RGB 反推色相，亮度/饱和度归零（黑、灰、白）时色相信息就丢了，再把滑块
   拖回来会变成红色。HSV 是编辑器需要的坐标系，hex 只是它的序列化形式。
   ========================================================================== */

/// 编辑器内部状态：色相 0..360、饱和度/亮度 0..1、不透明度 0..1。
#[derive(Clone, Copy, PartialEq, Debug)]
struct ColorState {
    h: f64,
    s: f64,
    v: f64,
    a: f64,
}

impl ColorState {
    /// 由 8 位通道构造。`prev` 只提供「灰阶时沿用哪个色相」。
    fn of(r: u8, g: u8, b: u8, a: f64, prev: Option<ColorState>) -> Self {
        let (h, s, v) = rgb_to_hsv(r, g, b);
        let h = if s <= 0.0 {
            prev.map(|p| p.h).unwrap_or(h)
        } else {
            h
        };
        Self {
            h,
            s,
            v,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// 解析 `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`；没有 alpha 位时按 1 处理。
    fn from_text(text: &str) -> Option<Self> {
        let (r, g, b, a) = parse_hex(text)?;
        Some(Self::of(r, g, b, a.unwrap_or(1.0), None))
    }

    fn rgb(&self) -> (u8, u8, u8) {
        hsv_to_rgb(self.h, self.s, self.v)
    }

    /// 无 alpha 时输出 `#rrggbb`，有 alpha 时输出 `#rrggbbaa` —— 值形状是契约，
    /// 调用方不该因为「拖了一下透明度条」就被迫处理两种长度。
    fn hex(&self) -> String {
        let (r, g, b) = self.rgb();
        let a8 = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        if a8 == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a8:02x}")
        }
    }

    fn hex_rgb(&self) -> String {
        let (r, g, b) = self.rgb();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// 可以直接进 style 的颜色串。通道之间用空格写法：`rgb(0 122 255 / 0.5)`
    /// 是 CSS Color 4 的形式，Chromium 一直支持，且不需要拼逗号分支。
    fn css(&self) -> String {
        let (r, g, b) = self.rgb();
        format!("rgb({r} {g} {b} / {:.3})", self.a.clamp(0.0, 1.0))
    }

    fn alpha_pct(&self) -> i32 {
        (self.a.clamp(0.0, 1.0) * 100.0).round() as i32
    }

    /// 两个状态在 8 位通道/1% 透明度上等价。外部值回流时用它判「是不是我自己
    /// 发出去的」，否则每次上报都会被 echo 回来、把拖拽中的 h 重置掉。
    fn same(&self, other: ColorState) -> bool {
        self.rgb() == other.rgb() && (self.a - other.a).abs() < 0.01
    }

    /// 灰阶（s == 0）没有色相，沿用上一次的 —— 否则拖到黑再拖回来会变红。
    fn keep_hue(self, prev: ColorState) -> Self {
        if self.s <= 0.0 && prev.s > 0.0 {
            Self { h: prev.h, ..self }
        } else {
            self
        }
    }
}

fn parse_hex(text: &str) -> Option<(u8, u8, u8, Option<f64>)> {
    let body = text.trim().strip_prefix('#')?;
    if body.is_empty() || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let nib = |c: char| c.to_digit(16).unwrap_or(0) as u8;
    let byte = |i: usize| u8::from_str_radix(&body[i..i + 2], 16).unwrap_or(0);
    let mut it = body.chars();
    match body.len() {
        3 => Some((
            nib(it.next()?) * 17,
            nib(it.next()?) * 17,
            nib(it.next()?) * 17,
            None,
        )),
        4 => Some((
            nib(it.next()?) * 17,
            nib(it.next()?) * 17,
            nib(it.next()?) * 17,
            Some(nib(it.next()?) as f64 / 15.0),
        )),
        6 => Some((byte(0), byte(2), byte(4), None)),
        8 => Some((
            byte(0),
            byte(2),
            byte(4),
            Some(byte(6) as f64 / 255.0),
        )),
        _ => None,
    }
}

/// 解析十六进制颜色并返回 8 位通道。给「只想知道 rgb」的调用方用（画廊里
/// 的实时读数就是它），解析失败返回 `None`。
pub fn hex_to_rgb(text: &str) -> Option<(u8, u8, u8)> {
    parse_hex(text).map(|(r, g, b, _)| (r, g, b))
}

/// 规范化十六进制写法：`#ABC` → `#aabbcc`，带 alpha 的保留 alpha。
/// 解析不了就原样回显 —— 显示层不该因为一个怪值变成空白。
pub fn normalize_hex(text: &str) -> String {
    ColorState::from_text(text)
        .map(|c| c.hex())
        .unwrap_or_else(|| text.trim().to_string())
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        (((g - b) / d) % 6.0) * 60.0
    } else if max == g {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let to8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to8(r), to8(g), to8(b))
}

/* ==========================================================================
   预设色
   ========================================================================== */

/// 一枚预设色：色值是**数据**，不是样式令牌，所以它出现在 Rust 里是合理的。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwatchItem {
    pub value: String,
    pub label: String,
}

impl SwatchItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }

    /// macOS 的八个系统强调色。`ColorPicker` 与 `Swatches` 的默认预设都用它。
    pub fn accents() -> Vec<Self> {
        [
            ("#0a84ff", "蓝色"),
            ("#bf5af2", "紫色"),
            ("#ff375f", "粉色"),
            ("#ff453a", "红色"),
            ("#ff9f0a", "橙色"),
            ("#ffd60a", "黄色"),
            ("#30d158", "绿色"),
            ("#8e8e93", "石墨"),
        ]
        .into_iter()
        .map(|(v, l)| Self::new(v, l))
        .collect()
    }
}

/* ==========================================================================
   平台能力：EyeDropper
   ========================================================================== */

fn eyedropper_supported() -> bool {
    web_sys::window()
        .map(|w| js_sys::Reflect::has(&w, &JsValue::from_str("EyeDropper")).unwrap_or(false))
        .unwrap_or(false)
}

/// 调起系统的屏幕取色器。
///
/// 没有用 `wasm-bindgen-futures`：本 crate 不加依赖，而 `EyeDropper.open()`
/// 返回的 Promise 可以在没有 async 运行时的情况下用 `then` 接 —— 只接一次，
/// 用 `Closure::once_into_js`（生命周期交给 JS，回调触发后自动释放）。
fn eyedropper_pick(apply: impl Fn(String) + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(ctor) = js_sys::Reflect::get(&window, &JsValue::from_str("EyeDropper")) else {
        return;
    };
    let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else {
        return;
    };
    let Ok(instance) = js_sys::Reflect::construct(&ctor, &js_sys::Array::new()) else {
        return;
    };
    let Ok(open) = js_sys::Reflect::get(&instance, &JsValue::from_str("open")) else {
        return;
    };
    let Ok(open) = open.dyn_into::<js_sys::Function>() else {
        return;
    };
    let Ok(promise) = open.call0(&instance) else {
        return;
    };
    let Ok(promise) = promise.dyn_into::<js_sys::Promise>() else {
        return;
    };

    let done = Closure::once_into_js(move |v: JsValue| {
        if let Ok(hex) = js_sys::Reflect::get(&v, &JsValue::from_str("sRGBHex")) {
            if let Some(hex) = hex.as_string() {
                apply(hex);
            }
        }
    });
    // 用户取消取色：静默恢复，不打断面板
    let fail = Closure::once_into_js(|_: JsValue| {});
    // 不走 js-sys 的 `Promise::then`：它现在要求 `ScopedClosure`，为一个
    // 一次性回调去建作用域不划算；`then` 本身就是个普通函数，动态取来调即可。
    let Ok(then) = js_sys::Reflect::get(&promise, &JsValue::from_str("then")) else {
        return;
    };
    let Ok(then) = then.dyn_into::<js_sys::Function>() else {
        return;
    };
    let _ = then.call2(
        &promise,
        done.unchecked_ref::<js_sys::Function>(),
        fail.unchecked_ref::<js_sys::Function>(),
    );
}

/* ==========================================================================
   取色器
   ========================================================================== */

/// 取色器：SV 面板 + 色相条 + 不透明度条 + HEX/RGB 输入 + 预设。
///
/// 受控值是 `#rrggbb` 或 `#rrggbbaa`；`on_change` **每次拖拽移动都上报**，
/// 订阅方可以拿它做实时预览。控件内部另存一份 HSV，用于「灰阶时保住色相」
/// 和「拖动中不被外部回流的 hex 重置」。
#[component]
pub fn ColorPicker(
    #[prop(into, optional)] value: MaybeProp<String>,
    /// 0 = 用 CSS 里的默认宽度（236px）
    #[prop(optional)] width: u32,
    /// 是否显示不透明度条
    #[prop(default = true)] alpha: bool,
    /// 预设色；留空用 `SwatchItem::accents()`
    #[prop(into, optional)] presets: Vec<SwatchItem>,
    #[prop(into, optional)] label: String,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    let sv_ref = NodeRef::<html::Div>::new();
    let hue_ref = NodeRef::<html::Div>::new();
    let alpha_ref = NodeRef::<html::Div>::new();

    // 无值时落到蓝色：面板总要显示一个颜色，空面板比蓝色更容易被当成坏了
    let seed = untrack(move || ColorState::from_text(&value.get().unwrap_or_default()))
        .unwrap_or(ColorState {
            h: 211.0,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        });
    let state = RwSignal::new(seed);
    // 0 = 没在拖；1 = SV；2 = 色相；3 = 不透明度
    let drag = RwSignal::new(0u8);
    // 输入框的「重写计数」：值没变但要把 DOM 里的文字规范化时，靠它触发重渲染
    let tick = RwSignal::new(0u32);
    // 哪一栏正在闪非法态：1 hex、2/3/4 分别对应 R/G/B
    let flash = RwSignal::new(0u8);

    // 外部值回流：只在「真的不是自己发出去的那个」时才覆盖本地 HSV
    Effect::new(move |_| {
        let raw = value.get().unwrap_or_default();
        let Some(next) = ColorState::from_text(&raw) else {
            return;
        };
        let cur = state.get_untracked();
        if cur.same(next) {
            return;
        }
        state.set(next.keep_hue(cur));
    });

    let emit = move |next: ColorState| {
        state.set(next);
        if let Some(cb) = on_change {
            cb.run(next.hex());
        }
    };
    let blocked = move || readonly;

    let bump = move || tick.update(|t| *t += 1);

    let flash_invalid = move |which: u8| {
        flash.set(which);
        let Some(window) = web_sys::window() else {
            return;
        };
        let cb = Closure::once_into_js(move || flash.set(0));
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.unchecked_ref::<js_sys::Function>(),
            520,
        );
    };

    // —— 指针：三条轨道共用一套「按下取值 → 捕获 → 松手」的节奏 ——
    let sv_apply = move |ev: &web_sys::PointerEvent| {
        let Some(el) = sv_ref.get() else { return };
        let r = el.get_bounding_client_rect();
        if r.width() <= 0.0 || r.height() <= 0.0 {
            return;
        }
        let s = ((ev.client_x() as f64 - r.left()) / r.width()).clamp(0.0, 1.0);
        let v = 1.0 - ((ev.client_y() as f64 - r.top()) / r.height()).clamp(0.0, 1.0);
        let cur = state.get_untracked();
        emit(ColorState { s, v, ..cur });
    };

    let hue_apply = move |ev: &web_sys::PointerEvent| {
        let Some(el) = hue_ref.get() else { return };
        let r = el.get_bounding_client_rect();
        if r.width() <= 0.0 {
            return;
        }
        let p = ((ev.client_x() as f64 - r.left()) / r.width()).clamp(0.0, 1.0);
        let cur = state.get_untracked();
        emit(ColorState {
            h: (p * 360.0).round(),
            ..cur
        });
    };

    let alpha_apply = move |ev: &web_sys::PointerEvent| {
        let Some(el) = alpha_ref.get() else { return };
        let r = el.get_bounding_client_rect();
        if r.width() <= 0.0 {
            return;
        }
        let p = ((ev.client_x() as f64 - r.left()) / r.width()).clamp(0.0, 1.0);
        let cur = state.get_untracked();
        emit(ColorState {
            a: (p * 100.0).round() / 100.0,
            ..cur
        });
    };

    // —— 输入框提交 ——
    let commit_hex = move |raw: &str| {
        if blocked() {
            bump();
            return;
        }
        match parse_hex(raw) {
            Some((r, g, b, a)) => {
                let cur = state.get_untracked();
                // 没写 alpha 的写法沿用当前不透明度：改颜色不该顺手把透明度重置
                let next = ColorState::of(r, g, b, a.unwrap_or(cur.a), Some(cur));
                if !cur.same(next) {
                    emit(next);
                }
            }
            None => flash_invalid(1),
        }
        bump();
    };

    let commit_channel = move |index: usize, raw: &str| {
        let cur = state.get_untracked();
        let (r, g, b) = cur.rgb();
        let mut ch = [r, g, b];
        let parsed = raw.trim().parse::<f64>().ok().filter(|n| n.is_finite());
        let Some(n) = parsed else {
            flash_invalid(2 + index as u8);
            bump();
            return;
        };
        ch[index] = n.round().clamp(0.0, 255.0) as u8;
        let next = ColorState::of(ch[0], ch[1], ch[2], cur.a, Some(cur));
        if !cur.same(next) && !blocked() {
            emit(next);
        }
        bump();
    };

    let presets = if presets.is_empty() {
        SwatchItem::accents()
    } else {
        presets
    };
    let aria_label = if label.is_empty() {
        "取色器".to_string()
    } else {
        label.clone()
    };
    let eyedropper_ok = eyedropper_supported();

    view! {
        <div
            class="c-colorpicker"
            role="group"
            aria-label=aria_label
            aria-readonly=readonly.then_some("true")
            style=move || {
                let w = if width == 0 {
                    String::new()
                } else {
                    format!("width:{width}px;")
                };
                format!("{w}--cw-color:{}", state.get().css())
            }
        >
            // SV 面板是二维的，ARIA 没有双轴滑块；role="application" 是唯一
            // 能表达「方向键在这里有自定义含义」的诚实选择（aria-valuenow 反而撒谎）。
            <div
                class="c-colorpicker__sv"
                role="application"
                tabindex=if readonly { "-1" } else { "0" }
                aria-label="饱和度与亮度（方向键移动，Shift 加速）"
                style=move || format!("--h:{}", state.get().h.round())
                node_ref=sv_ref
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    if blocked() {
                        return;
                    }
                    ev.prevent_default();
                    if let Some(el) = sv_ref.get() {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                        let _ = el.focus();
                    }
                    drag.set(1);
                    sv_apply(&ev);
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    if drag.get() == 1 {
                        sv_apply(&ev);
                    }
                }
                on:pointerup=move |_| {
                    if drag.get_untracked() == 1 {
                        drag.set(0);
                    }
                }
                on:pointercancel=move |_| { drag.set(0); }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if blocked() {
                        return;
                    }
                    let step = if ev.shift_key() { 0.1 } else { 0.01 };
                    let cur = state.get_untracked();
                    let next = match ev.key().as_str() {
                        "ArrowLeft" => ColorState { s: (cur.s - step).clamp(0.0, 1.0), ..cur },
                        "ArrowRight" => ColorState { s: (cur.s + step).clamp(0.0, 1.0), ..cur },
                        "ArrowUp" => ColorState { v: (cur.v + step).clamp(0.0, 1.0), ..cur },
                        "ArrowDown" => ColorState { v: (cur.v - step).clamp(0.0, 1.0), ..cur },
                        _ => return,
                    };
                    ev.prevent_default();
                    emit(next);
                }
            >
                <span class="c-colorpicker__sv-white"></span>
                <span class="c-colorpicker__sv-black"></span>
                <span
                    class="c-colorpicker__thumb"
                    aria-hidden="true"
                    style=move || {
                        let c = state.get();
                        format!("left:{:.3}%;top:{:.3}%", c.s * 100.0, (1.0 - c.v) * 100.0)
                    }
                ></span>
            </div>

            <div
                class="c-colorpicker__hue"
                role="slider"
                tabindex=if readonly { "-1" } else { "0" }
                aria-label="色相"
                aria-orientation="horizontal"
                aria-valuemin="0"
                aria-valuemax="360"
                aria-valuenow=move || state.get().h.round().to_string()
                aria-valuetext=move || format!("{} 度", state.get().h.round())
                node_ref=hue_ref
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    if blocked() {
                        return;
                    }
                    ev.prevent_default();
                    if let Some(el) = hue_ref.get() {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                        let _ = el.focus();
                    }
                    drag.set(2);
                    hue_apply(&ev);
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    if drag.get() == 2 {
                        hue_apply(&ev);
                    }
                }
                on:pointerup=move |_| {
                    if drag.get_untracked() == 2 {
                        drag.set(0);
                    }
                }
                on:pointercancel=move |_| { drag.set(0); }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if blocked() {
                        return;
                    }
                    let step = if ev.shift_key() { 10.0 } else { 1.0 };
                    let cur = state.get_untracked();
                    let h = match ev.key().as_str() {
                        "ArrowLeft" | "ArrowDown" => cur.h - step,
                        "ArrowRight" | "ArrowUp" => cur.h + step,
                        "Home" => 0.0,
                        "End" => 360.0,
                        _ => return,
                    };
                    ev.prevent_default();
                    emit(ColorState { h: h.clamp(0.0, 360.0), ..cur });
                }
            >
                <span
                    class="c-colorpicker__thumb"
                    aria-hidden="true"
                    style=move || format!("left:{:.3}%", state.get().h / 360.0 * 100.0)
                ></span>
            </div>

            {alpha.then(|| view! {
                <div
                    class="c-colorpicker__alpha"
                    role="slider"
                    tabindex=if readonly { "-1" } else { "0" }
                    aria-label="不透明度"
                    aria-orientation="horizontal"
                    aria-valuemin="0"
                    aria-valuemax="100"
                    aria-valuenow=move || state.get().alpha_pct().to_string()
                    aria-valuetext=move || format!("{}%", state.get().alpha_pct())
                    node_ref=alpha_ref
                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                        if blocked() {
                            return;
                        }
                        ev.prevent_default();
                        if let Some(el) = alpha_ref.get() {
                            let _ = el.set_pointer_capture(ev.pointer_id());
                            let _ = el.focus();
                        }
                        drag.set(3);
                        alpha_apply(&ev);
                    }
                    on:pointermove=move |ev: web_sys::PointerEvent| {
                        if drag.get() == 3 {
                            alpha_apply(&ev);
                        }
                    }
                    on:pointerup=move |_| {
                        if drag.get_untracked() == 3 {
                            drag.set(0);
                        }
                    }
                    on:pointercancel=move |_| { drag.set(0); }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if blocked() {
                            return;
                        }
                        let step = if ev.shift_key() { 0.1 } else { 0.01 };
                        let cur = state.get_untracked();
                        let a = match ev.key().as_str() {
                            "ArrowLeft" | "ArrowDown" => cur.a - step,
                            "ArrowRight" | "ArrowUp" => cur.a + step,
                            "Home" => 0.0,
                            "End" => 1.0,
                            _ => return,
                        };
                        ev.prevent_default();
                        emit(ColorState { a: a.clamp(0.0, 1.0), ..cur });
                    }
                >
                    <span
                        class="c-colorpicker__alpha-fill"
                        aria-hidden="true"
                        style=move || {
                            let (r, g, b) = state.get().rgb();
                            format!(
                                "background:linear-gradient(to right,rgb({r} {g} {b} / 0),rgb({r} {g} {b} / 1))"
                            )
                        }
                    ></span>
                    <span
                        class="c-colorpicker__thumb"
                        aria-hidden="true"
                        style=move || format!("left:{:.3}%", state.get().a * 100.0)
                    ></span>
                </div>
            })}

            <div class="c-colorpicker__row">
                <input
                    class="c-colorpicker__hex"
                    type="text"
                    autocomplete="off"
                    spellcheck="false"
                    maxlength="9"
                    aria-label="十六进制颜色值"
                    data-invalid=move || (flash.get() == 1).then_some("1")
                    value=move || {
                        tick.get();
                        state.get().hex()
                    }
                    prop:value=move || {
                        tick.get();
                        state.get().hex()
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.is_composing() || ev.key_code() == 229 {
                            return;
                        }
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            commit_hex(&event_target_value(&ev));
                        }
                    }
                    on:change=move |ev| commit_hex(&event_target_value(&ev))
                />
                <button
                    class="c-colorpicker__eyedropper"
                    type="button"
                    aria-label="从屏幕取色"
                    title=if eyedropper_ok {
                        "从屏幕取色"
                    } else {
                        "当前浏览器不支持屏幕取色"
                    }
                    disabled=!eyedropper_ok
                    on:click=move |_| {
                        if blocked() {
                            return;
                        }
                        // 取色只改颜色，不动不透明度：EyeDropper 只回 #rrggbb
                        let a = state.get_untracked().a;
                        eyedropper_pick(move |hex| {
                            if let Some(next) = ColorState::from_text(&hex) {
                                emit(ColorState { a, ..next });
                                bump();
                            }
                        });
                    }
                >
                    <Icon name=IconName::EYE size=14 />
                </button>
            </div>

            <div class="c-colorpicker__row c-colorpicker__rgbrow">
                {["红", "绿", "蓝"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| {
                        view! {
                            <label class="c-colorpicker__field">
                                <span class="c-colorpicker__fieldlabel">{name}</span>
                                <input
                                    class="c-colorpicker__num"
                                    type="text"
                                    inputmode="numeric"
                                    autocomplete="off"
                                    maxlength="3"
                                    aria-label=format!("{name}色通道，0 到 255")
                                    data-invalid=move || (flash.get() == 2 + i as u8).then_some("1")
                                    value=move || {
                                        tick.get();
                                        let (r, g, b) = state.get().rgb();
                                        [r, g, b][i].to_string()
                                    }
                                    prop:value=move || {
                                        tick.get();
                                        let (r, g, b) = state.get().rgb();
                                        [r, g, b][i].to_string()
                                    }
                                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                                        if ev.is_composing() || ev.key_code() == 229 {
                                            return;
                                        }
                                        if ev.key() == "Enter" {
                                            ev.prevent_default();
                                            commit_channel(i, &event_target_value(&ev));
                                        }
                                    }
                                    on:change=move |ev| commit_channel(i, &event_target_value(&ev))
                                />
                            </label>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="c-colorpicker__presets" role="group" aria-label="预设颜色">
                {presets
                    .iter()
                    .map(|p| {
                        let pressed = {
                            let value = p.value.clone();
                            move || {
                                state
                                    .get()
                                    .hex_rgb()
                                    .eq_ignore_ascii_case(value.trim())
                                    .to_string()
                            }
                        };
                        let hex = p.value.clone();
                        view! {
                            <button
                                class="c-colorpicker__preset"
                                type="button"
                                aria-pressed=pressed
                                aria-label=p.label.clone()
                                title=p.label.clone()
                                data-hex=p.value.to_lowercase()
                                style=format!("--sw:{}", p.value)
                                on:click=move |_| {
                                    if let Some(next) = ColorState::from_text(&hex) {
                                        if !blocked() {
                                            let cur = state.get_untracked();
                                            emit(ColorState { a: cur.a, ..next }.keep_hue(cur));
                                            bump();
                                        }
                                    }
                                }
                            ></button>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="c-colorpicker__foot">
                <span class="c-colorpicker__hextext selectable">{move || state.get().hex()}</span>
                <span class="c-colorpicker__space">{move || {
                    let c = state.get();
                    if c.a >= 0.999 {
                        "sRGB".to_string()
                    } else {
                        format!("sRGB · {}%", c.alpha_pct())
                    }
                }}</span>
            </div>
        </div>
    }
}
