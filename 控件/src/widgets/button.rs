use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::base::types::Size;

/// 按钮外观。`Secondary` 是默认（不写类）。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonVariant {
    #[default]
    Secondary,
    Primary,
    Tonal,
    Plain,
    Danger,
}

impl ButtonVariant {
    fn class(self) -> Option<&'static str> {
        match self {
            Self::Secondary => None,
            Self::Primary => Some("c-btn--primary"),
            Self::Tonal => Some("c-btn--tonal"),
            Self::Plain => Some("c-btn--plain"),
            Self::Danger => Some("c-btn--danger"),
        }
    }
}

#[component]
pub fn Button(
    #[prop(into, optional)] label: String,
    #[prop(optional)] icon: Option<IconName>,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: Size,
    #[prop(into, optional)] disabled: MaybeProp<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let mut cls = String::from("c-btn");
    if let Some(v) = variant.class() {
        cls.push(' ');
        cls.push_str(v);
    }
    if let Some(s) = size.class() {
        cls.push_str(" c-btn--");
        cls.push_str(s);
    }
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let icon_px = size.icon_px();
    let has_label = !label.is_empty();

    view! {
        <button
            class=cls
            type="button"
            disabled=move || disabled.get().unwrap_or(false)
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {icon.map(|name| view! { <Icon name=name size=icon_px /> })}
            {has_label.then(|| view! { <span class="c-btn__label">{label.clone()}</span> })}
        </button>
    }
}

/// 图标按钮外观。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum IconButtonVariant {
    #[default]
    Plain,
    Tonal,
    On,
}

impl IconButtonVariant {
    fn class(self) -> Option<&'static str> {
        match self {
            Self::Plain => None,
            Self::Tonal => Some("c-iconbtn--tonal"),
            Self::On => Some("c-iconbtn--on"),
        }
    }
}

#[component]
pub fn IconButton(
    #[prop(into)] icon: IconName,
    #[prop(into, optional)] label: String,
    #[prop(optional)] variant: IconButtonVariant,
    #[prop(optional)] size: Size,
    #[prop(optional)] active: bool,
    #[prop(optional)] toggle: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let mut cls = String::from("c-iconbtn");
    if let Some(v) = variant.class() {
        cls.push(' ');
        cls.push_str(v);
    }
    if let Some(s) = size.class() {
        cls.push_str(" c-iconbtn--");
        cls.push_str(s);
    }
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let icon_px = match size {
        Size::Sm => 13,
        Size::Md => 16,
        Size::Lg => 18,
    };
    let aria_pressed = toggle.then(|| active.to_string());

    view! {
        <button
            class=cls
            type="button"
            title=label.clone()
            aria-label=label
            aria-pressed=aria_pressed
            disabled=disabled
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            <Icon name=icon size=icon_px />
        </button>
    }
}
