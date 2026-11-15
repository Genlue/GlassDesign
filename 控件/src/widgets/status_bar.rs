//! 状态栏：窗口底部的一条常驻底栏，左边只读状态，右边进度。
//!
//! 边界（与 60-status-bar.css 的分工）：
//! - 它是**内容面**，不是玻璃面：贴边长期占位，背后没有可透的东西，
//!   所以只有一层 `--surface-content` + 一条上边发丝。组件不写任何样式值。
//! - 组件**不改状态**。可点的状态项只把 `()` 报给调用方，进度条只显示调用方给的
//!   值 —— 组件自己也存一份就会出现「按钮说已取消、数据说还在跑」。
//! - 60 分区里还有一整套任务浮层（`__tasks` / `__more` / `__panel` / `__row` /
//!   `__action` / `__live`）。那套需要浮层宿主与朗读节流，属于后续批次，
//!   这里**不造 API**：类名留着，等 `OverlayHost` 收口后再补。

use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

/// 状态项的 live 指示语义。只有给了 `live` 才画那颗点。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusTone {
    Accent,
    Success,
    Warning,
    Danger,
}

impl StatusTone {
    fn key(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

/// 底栏容器。`right` 是右侧槽（进度、版本号之类），给了才占位。
#[component]
pub fn StatusBar(children: Children, #[prop(optional)] right: Option<Children>) -> impl IntoView {
    view! {
        <div class="c-statusbar" role="region" aria-label="状态栏">
            <div class="c-statusbar__items">
                <div class="c-statusbar__items-start">{children()}</div>
                {right.map(|r| view! { <div class="c-statusbar__items-end">{r()}</div> })}
            </div>
        </div>
    }
}

/// 一条状态：图标 + 标签 + 值，可选 live 点。
///
/// 给了 `on_click` 才挂 `role="button"` / `tabindex`：不可点的状态项不该进 Tab 序列，
/// 也不该在悬停时亮起来。
#[component]
pub fn StatusItem(
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(optional)] live: Option<StatusTone>,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    let tone = live.map(StatusTone::key);
    let interactive = on_click.is_some();

    view! {
        <div
            class="c-statusbar__item"
            data-tone=tone
            role=interactive.then_some("button")
            tabindex=interactive.then_some("0")
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if !interactive {
                    return;
                }
                if ev.key() == "Enter" || ev.key() == " " {
                    ev.prevent_default();
                    if let Some(cb) = on_click {
                        cb.run(());
                    }
                }
            }
        >
            {icon.map(|name| view! {
                <span class="c-statusbar__item-icon"><Icon name=name size=12 /></span>
            })}
            <span class="c-statusbar__item-label">{label}</span>
            <span class="c-statusbar__item-value">{value}</span>
            {tone.map(|_| view! { <span class="c-statusbar__dot" aria-hidden="true"></span> })}
        </div>
    }
}

/// 右侧进度：标签 + 4px 胶囊条 + 百分比 + 阶段标记。
///
/// `value` 是 **0..1 的比例**（与旧库 `panels/status-bar.js` 的 `progress` 一致，
/// 也和本次 `SplitView` 的比例口径一致）；`aria-valuenow` 和可见百分比由它算出来。
///
/// `phase` 取 `running` / `paused` / `success` / `error` / `cancelled` / `queued`
/// —— 就是 60 分区 `[data-phase]` 认的那几个词，组件原样写进 `data-phase`。
/// 未知值不影响布局，只是拿不到对应颜色（标记点退化成中性圆点）。
#[component]
pub fn StatusProgress(
    #[prop(into)] label: String,
    #[prop(default = 0.0)] value: f64,
    #[prop(into)] phase: String,
) -> impl IntoView {
    let pct = move || {
        let v = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
        (v * 100.0).round() as i32
    };
    // 进行中不画「完成」图标：那颗对勾会让人以为跑完了。
    let marker = match phase.as_str() {
        "success" => Some(IconName::CIRCLE_CHECK),
        "error" => Some(IconName::TRIANGLE_ALERT),
        "paused" => Some(IconName::PAUSE),
        "cancelled" => Some(IconName::BAN),
        _ => None,
    };
    let phase_attr = phase.clone();

    view! {
        <div class="c-statusbar__task" data-phase=phase_attr>
            <span class="c-statusbar__phase" aria-hidden="true">
                {match marker {
                    Some(name) => view! { <Icon name=name size=13 /> }.into_any(),
                    None => view! { <span class="c-statusbar__dot"></span> }.into_any(),
                }}
            </span>
            <span
                class="c-statusbar__task-label"
                title=label.clone()
            >{label.clone()}</span>
            <span
                class="c-statusbar__progress"
                role="progressbar"
                aria-label=label
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow=move || pct().to_string()
            >
                <span
                    class="c-statusbar__progress-fill"
                    style=move || format!("width:{}%", pct())
                ></span>
            </span>
            <span class="c-statusbar__pct">{move || format!("{}%", pct())}</span>
        </div>
    }
}
