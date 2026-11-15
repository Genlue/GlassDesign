use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

fn clamp01(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 100.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum RingSize {
    #[default]
    Md,
    Lg,
}

#[component]
pub fn ProgressRing(
    #[prop(into, optional)] value: MaybeProp<f64>,
    #[prop(optional)] size: RingSize,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let now = move || clamp01(value.get().unwrap_or(0.0));
    let cls = if size == RingSize::Lg { "c-ring c-ring--lg" } else { "c-ring" };
    let style = move || format!("--v:{}", now() / 100.0);

    view! {
        <svg
            class=cls
            viewBox="0 0 36 36"
            role="progressbar"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow=move || now().to_string()
            aria-label=label
            style=style
        >
            <circle class="c-ring__track" cx="18" cy="18" r="15.9155"></circle>
            <circle class="c-ring__bar" cx="18" cy="18" r="15.9155"></circle>
        </svg>
    }
}

#[component]
pub fn Progress(
    #[prop(into, optional)] value: MaybeProp<f64>,
    #[prop(optional)] max: f64,
    #[prop(into, optional)] label: String,
    #[prop(optional)] indeterminate: bool,
) -> impl IntoView {
    let max = if max <= 0.0 { 100.0 } else { max };
    let pct = move || {
        if indeterminate {
            100.0
        } else {
            (clamp01(value.get().unwrap_or(0.0)) / max * 100.0).clamp(0.0, 100.0)
        }
    };

    view! {
        <div
            class="c-progress"
            role="progressbar"
            aria-label=label
            aria-valuemin="0"
            aria-valuemax=max.to_string()
            aria-valuenow=move || (!indeterminate).then(|| value.get().unwrap_or(0.0).to_string())
            aria-valuetext=indeterminate.then_some("进行中")
            data-indeterminate=indeterminate.then_some("1")
        >
            <div class="c-progress__bar" style=move || format!("width:{}%", pct())></div>
        </div>
    }
}

#[component]
pub fn Spinner(#[prop(into, optional)] label: String) -> impl IntoView {
    let label = if label.is_empty() { "加载中".to_string() } else { label };
    view! { <span class="c-spinner" role="status" aria-label=label></span> }
}

#[component]
pub fn Skeleton(
    #[prop(into, optional)] width: String,
    #[prop(into, optional)] height: String,
    #[prop(optional)] lines: usize,
    #[prop(optional)] circle: bool,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let width = if width.is_empty() { "100%".to_string() } else { width };
    let height = if height.is_empty() { "12px".to_string() } else { height };
    let mut base = String::from("c-skeleton");
    if circle {
        base.push_str(" c-skeleton--circle");
    }
    base.push_str(" c-skeleton--text");
    if !class.is_empty() {
        base.push(' ');
        base.push_str(&class);
    }

    if lines > 1 {
        let rows = (0..lines)
            .map(|i| {
                let w = if i + 1 == lines { "62%" } else { "100%" };
                view! {
                    <div class=base.clone() style=format!("width:{w};height:{height}")></div>
                }
            })
            .collect::<Vec<_>>();
        view! {
            <div class="l-stack l-stack--3" style=format!("width:{width}")>{rows}</div>
        }
        .into_any()
    } else {
        view! {
            <div class=base style=format!("width:{width};height:{height}")></div>
        }
        .into_any()
    }
}

#[component]
pub fn Empty(
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] desc: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="c-empty">
            <span class="c-empty__icon">
                <Icon name=icon.unwrap_or(IconName::SQUARE_DASHED) size=32 />
            </span>
            {(!title.is_empty()).then(|| view! { <div class="c-empty__title">{title}</div> })}
            {(!desc.is_empty()).then(|| view! { <div class="c-empty__desc">{desc}</div> })}
            {children.map(|c| c())}
        </div>
    }
}
