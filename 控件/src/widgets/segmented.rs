use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::base::overlay::schedule;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SegmentedOption {
    pub value: String,
    pub label: String,
    pub icon: Option<IconName>,
    pub title: Option<String>,
}

impl SegmentedOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon: None,
            title: None,
        }
    }
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SegmentedSize {
    #[default]
    Regular,
    Lg,
}

#[component]
pub fn Segmented(
    #[prop(into)] options: Vec<SegmentedOption>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] size: SegmentedSize,
) -> impl IntoView {
    let cls = if size == SegmentedSize::Lg {
        "c-seg c-seg--lg"
    } else {
        "c-seg"
    };
    let options_for_click = options.clone();
    let root_ref = NodeRef::<html::Div>::new();
    let ind_ref = NodeRef::<html::Span>::new();

    // 把选中块摆到当前选中项上。选中项变了要重摆，窗口宽度变了也要重摆
    // （同一个值在窄窗口里可能换行，几何就全变了）。
    let place = move || {
        let (Some(root), Some(ind)) = (root_ref.get_untracked(), ind_ref.get_untracked()) else {
            return;
        };
        let Ok(Some(opt)) = root.query_selector(".c-seg__opt[aria-checked=\"true\"]") else {
            return;
        };
        let rb = root.get_bounding_client_rect();
        let ob = opt.get_bounding_client_rect();
        if ob.width() == 0.0 && ob.height() == 0.0 {
            return;
        }
        let style: web_sys::HtmlElement = ind.unchecked_into();
        let style = style.style();
        let _ = style.set_property(
            "transform",
            &format!(
                "translate({}px, {}px)",
                ob.left() - rb.left(),
                ob.top() - rb.top()
            ),
        );
        let _ = style.set_property("width", &format!("{}px", ob.width()));
        let _ = style.set_property("height", &format!("{}px", ob.height()));
    };

    Effect::new(move |_| {
        // 显式追踪选中值：换选项 → 重摆。
        let _ = value.get();
        // 等一帧：Leptos 这轮渲染还没落定时量到的是 0 宽。
        schedule(place);
    });

    // 尺寸监听挂在 window 上（控件本身没有 ResizeObserver 依赖，
    // 而且真正会变的是外层容器宽度 —— 胶囊宽度由内容决定）。
    let listener = window_event_listener(leptos::ev::resize, move |_| place());
    on_cleanup(move || drop(listener));

    let buttons = options.into_iter().map(|opt| {
        let value_for_state = opt.value.clone();
        let value_for_tab = opt.value.clone();
        let value_for_click = opt.value.clone();
        let on_change = on_change;
        let options = options_for_click.clone();
        let title = opt.title.clone();
        view! {
            <button
                class="c-seg__opt"
                type="button"
                role="radio"
                data-value=opt.value.clone()
                title=title
                aria-checked=move || if value.get().as_deref() == Some(value_for_state.as_str()) { "true" } else { "false" }
                tabindex=move || if value.get().as_deref() == Some(value_for_tab.as_str()) { "0" } else { "-1" }
                on:click=move |_| {
                    if value.get().as_deref() == Some(value_for_click.as_str()) {
                        return;
                    }
                    if let Some(cb) = on_change {
                        cb.run(value_for_click.clone());
                    }
                }
                on:keydown={
                    let options = options.clone();
                    let current = opt.value.clone();
                    move |ev: web_sys::KeyboardEvent| {
                        let step = match ev.key().as_str() {
                            "ArrowRight" | "ArrowDown" => 1isize,
                            "ArrowLeft" | "ArrowUp" => -1,
                            _ => return,
                        };
                        ev.prevent_default();
                        let Some(i) = options.iter().position(|o| o.value == current) else { return };
                        let next = (i as isize + step).rem_euclid(options.len() as isize) as usize;
                        if let Some(cb) = on_change {
                            cb.run(options[next].value.clone());
                        }
                    }
                }
            >
                {opt.icon.map(|name| view! { <Icon name=name size=14 /> })}
                {(!opt.label.is_empty()).then(|| view! { <span class="c-seg__label">{opt.label}</span> })}
            </button>
        }
    }).collect::<Vec<_>>();

    view! {
        <div
            class=cls
            role="radiogroup"
            data-value=move || value.get().unwrap_or_default()
            node_ref=root_ref
        >
            // 选中块：一枚会滑动的强调色滑块。
            // CSS 只给形状，几何（x / y / 宽 / 高）必须实测后写进内联样式 ——
            // 选项宽度随文案变，写死任何一个数都会在中文/英文之间错位。
            // **少了这一层，选中项只剩 `--text-on-accent` 的白字，浅色主题下直接消失。**
            <span class="c-seg__ind" node_ref=ind_ref></span>
            {buttons}
        </div>
    }
}
