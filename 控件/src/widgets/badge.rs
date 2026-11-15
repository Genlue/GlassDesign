use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum BadgeVariant {
    #[default]
    Neutral,
    Accent,
    Danger,
}

impl BadgeVariant {
    fn class(self) -> Option<&'static str> {
        match self {
            Self::Neutral => None,
            Self::Accent => Some("c-badge--accent"),
            Self::Danger => Some("c-badge--danger"),
        }
    }
}

#[component]
pub fn Badge(
    #[prop(into)] label: String,
    #[prop(optional)] variant: BadgeVariant,
    #[prop(optional)] icon: Option<IconName>,
) -> impl IntoView {
    let cls = match variant.class() {
        Some(extra) => format!("c-badge {extra}"),
        None => "c-badge".to_string(),
    };
    view! {
        <span class=cls>
            {icon.map(|name| view! { <Icon name=name size=11 /> })}
            {label}
        </span>
    }
}
