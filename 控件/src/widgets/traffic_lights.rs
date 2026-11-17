use leptos::prelude::*;

/// 未保存圆点 / 悬停图形。三灯共用一套几何：28 单位栅格画 14pt 圆。
/// 外圈是「描边环」不是实心圆 —— 实心圆在半透明本体下会透出来，失焦那套
/// 灰度会叠成两倍深（原库踩过）。
const GLYPH_CLOSE: &str = "M9 9l10 10M19 9L9 19";
const GLYPH_MIN: &str = "M8 14h12";
const GLYPH_ZOOM: &str = "M8 8h10L8 18zM20 20H10l10-10z";

/// 窗口红绿灯（macOS 三灯）。
///
/// 只做外观与语义：`on_close` / `on_minimize` / `on_zoom` 不传就是纯展示。
/// 宿主（Tauri）自己接窗口操作 —— 控件库不依赖 `tauri` crate。
#[component]
pub fn TrafficLights(
    /// 窗口失焦：三灯统一退成中性灰，且不出图形
    #[prop(into, optional)] focused: MaybeProp<bool>,
    /// 有未保存修改：红灯中心一枚实心圆点
    #[prop(optional)] dirty: bool,
    /// 规范展示用：不悬停也显示图形
    #[prop(optional)] reveal: bool,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] on_minimize: Option<Callback<()>>,
    #[prop(optional)] on_zoom: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div
            class="c-traffic"
            data-focused=move || {
                if focused.get().unwrap_or(true) { "1" } else { "0" }
            }
            data-dirty=if dirty { "1" } else { "0" }
            data-reveal=if reveal { "1" } else { "0" }
        >
            <Light
                kind="close"
                title="关闭"
                label="关闭窗口"
                glyph=GLYPH_CLOSE
                stroke=4.5
                filled=false
                busy=dirty
                on_click=on_close
            />
            <Light
                kind="min"
                title="最小化"
                label="最小化窗口"
                glyph=GLYPH_MIN
                stroke=4.0
                filled=false
                busy=false
                on_click=on_minimize
            />
            <Light
                kind="zoom"
                title="最大化"
                label="最大化窗口"
                glyph=GLYPH_ZOOM
                stroke=0.0
                filled=true
                busy=false
                on_click=on_zoom
            />
        </div>
    }
}

#[component]
fn Light(
    kind: &'static str,
    title: &'static str,
    label: &'static str,
    glyph: &'static str,
    stroke: f64,
    filled: bool,
    busy: bool,
    on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <button
            class=format!("c-traffic__light c-traffic__light--{kind}")
            type="button"
            title=title
            aria-label=label
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            <svg class="c-traffic__svg" viewBox="0 0 28 28" aria-hidden="true" focusable="false">
                <circle class="c-traffic__rim" cx="14" cy="14" r="13.5"></circle>
                <circle class="c-traffic__body" cx="14" cy="14" r="13"></circle>
                {busy.then(|| view! { <circle class="c-traffic__dot" cx="14" cy="14" r="5"></circle> })}
                <g class="c-traffic__glyph">
                    <path
                        d=glyph
                        fill=if filled { "currentColor" } else { "none" }
                        stroke=if filled { "none" } else { "currentColor" }
                        stroke-width=format!("{stroke}")
                        stroke-linecap="round"
                    ></path>
                </g>
            </svg>
        </button>
    }
}
