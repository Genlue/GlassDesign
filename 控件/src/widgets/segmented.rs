use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::anchor::ResizeObserverHandle;
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
    #[prop(into, optional)] label: String,
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
        let (Some(root), Some(ind)) = (
            root_ref.try_get_untracked().flatten(),
            ind_ref.try_get_untracked().flatten(),
        ) else {
            return;
        };
        if !root.is_connected() {
            return;
        }
        let ind: web_sys::HtmlElement = ind.unchecked_into();
        let style = ind.style();
        let Ok(Some(opt)) = root.query_selector(".c-seg__opt[aria-checked=\"true\"]") else {
            let _ = style.set_property("width", "0px");
            let _ = ind.class_list().add_1("is-static");
            return;
        };
        let opt: web_sys::HtmlElement = opt.unchecked_into();
        // Geometry in the indicator's containing block, not viewport-scaled rects.
        let (width, height) = (opt.offset_width(), opt.offset_height());
        if width == 0 || height == 0 {
            return;
        }
        let _ = style.set_property(
            "transform",
            &format!("translate({}px, {}px)", opt.offset_left(), opt.offset_top()),
        );
        let _ = style.set_property("width", &format!("{width}px"));
        let _ = style.set_property("height", &format!("{height}px"));
        // Commit the first position without animating from a zero-width block.
        if ind.class_list().contains("is-static") {
            let _ = ind.offset_width();
            let _ = ind.class_list().remove_1("is-static");
        }
    };

    Effect::new(move |_| {
        // 显式追踪选中值：换选项 → 重摆。
        let _ = value.get();
        // 等一帧：Leptos 这轮渲染还没落定时量到的是 0 宽。
        schedule(place);
    });

    // Track option sizes as well as the container: a font or host CSS change
    // can move the selected option without resizing the outer track.
    Effect::new(move |_| {
        let Some(root) = root_ref.get() else { return };
        let mut targets = vec![root.clone().unchecked_into::<web_sys::HtmlElement>()];
        if let Ok(options) = root.query_selector_all(".c-seg__opt") {
            for i in 0..options.length() {
                if let Some(option) = options.item(i) {
                    targets.push(option.unchecked_into());
                }
            }
        }
        if let Some(observer) = ResizeObserverHandle::new(targets, place) {
            let observer = StoredValue::new_local(Some(observer));
            on_cleanup(move || observer.set_value(None));
        }
    });

    // Keep resize handling for environments without ResizeObserver.
    let listener = window_event_listener(leptos::ev::resize, move |_| place());
    on_cleanup(move || listener.remove());

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
                            if let Some(root) = root_ref.try_get_untracked().flatten() {
                                if let Some(button) = root.query_selector_all(".c-seg__opt").ok()
                                    .and_then(|buttons| buttons.item(next as u32))
                                    .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
                                {
                                    let _ = button.focus();
                                }
                            }
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
            aria-label=(!label.is_empty()).then_some(label)
            data-value=move || value.get().unwrap_or_default()
            node_ref=root_ref
        >
            // 选中块：一枚会滑动的强调色滑块。
            // CSS 只给形状，几何（x / y / 宽 / 高）必须实测后写进内联样式 ——
            // 选项宽度随文案变，写死任何一个数都会在中文/英文之间错位。
            // **少了这一层，选中项只剩 `--text-on-accent` 的白字，浅色主题下直接消失。**
            <span class="c-seg__ind is-static" node_ref=ind_ref></span>
            {buttons}
        </div>
    }
}
