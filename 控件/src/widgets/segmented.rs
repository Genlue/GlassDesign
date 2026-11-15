use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

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
    let cls = if size == SegmentedSize::Lg { "c-seg c-seg--lg" } else { "c-seg" };
    let options_for_click = options.clone();

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
        <div class=cls role="radiogroup" data-value=move || value.get().unwrap_or_default()>
            {buttons}
        </div>
    }
}
