use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Tone {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Self::Info => "c-inline--info",
            Self::Success => "c-inline--success",
            Self::Warning => "c-inline--warning",
            Self::Danger => "c-inline--danger",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Info => IconName::INFO,
            Self::Success => IconName::CIRCLE_CHECK,
            Self::Warning => IconName::TRIANGLE_ALERT,
            Self::Danger => IconName::X_CIRCLE,
        }
    }
}

#[component]
pub fn InlineMessage(
    #[prop(optional)] tone: Tone,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] description: String,
    #[prop(optional)] dismissible: bool,
    #[prop(optional)] on_dismiss: Option<Callback<()>>,
) -> impl IntoView {
    let has_title = !title.is_empty();
    let has_desc = !description.is_empty();
    let role = if tone == Tone::Danger {
        "alert"
    } else {
        "status"
    };
    let live = if tone == Tone::Danger {
        None
    } else {
        Some("polite")
    };

    view! {
        <div class=format!("c-inline {}", tone.class()) role=role aria-live=live>
            <span class="c-inline__icon" aria-hidden="true">
                <Icon name=tone.icon() size=16 />
            </span>
            <div class="c-inline__body">
                <div class="c-inline__title" hidden=!has_title>{title}</div>
                <div class="c-inline__desc" hidden=!has_desc>{description}</div>
            </div>
            {dismissible.then(|| view! {
                <button
                    class="c-inline__close"
                    type="button"
                    aria-label="关闭"
                    on:click=move |_| { if let Some(cb) = on_dismiss { cb.run(()); } }
                >
                    <Icon name=IconName::X size=12 />
                </button>
            })}
        </div>
    }
}
