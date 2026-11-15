use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AvatarSize {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl AvatarSize {
    fn class(self) -> Option<&'static str> {
        match self {
            Self::Sm => Some("c-avatar--sm"),
            Self::Md => None,
            Self::Lg => Some("c-avatar--lg"),
            Self::Xl => Some("c-avatar--xl"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AvatarStatus {
    Online,
    Away,
    Busy,
    Offline,
}

impl AvatarStatus {
    fn class(self) -> &'static str {
        match self {
            Self::Online => "c-avatar__status--online",
            Self::Away => "c-avatar__status--away",
            Self::Busy => "c-avatar__status--busy",
            Self::Offline => "c-avatar__status--offline",
        }
    }
}

pub(crate) fn initials(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let first = trimmed.chars().next().unwrap_or(' ');
    if (first as u32) >= 0x3400 {
        return first.to_string();
    }
    let mut out = String::new();
    for word in trimmed.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    out
}

#[component]
pub fn Avatar(
    #[prop(into, optional)] src: String,
    #[prop(into, optional)] name: String,
    #[prop(optional)] size: AvatarSize,
    #[prop(optional)] square: bool,
    #[prop(optional)] status: Option<AvatarStatus>,
) -> impl IntoView {
    let mut cls = String::from("c-avatar");
    if let Some(s) = size.class() {
        cls.push(' ');
        cls.push_str(s);
    }
    if square {
        cls.push_str(" c-avatar--square");
    }
    let text = initials(&name);
    let alt = name.clone();
    let has_src = !src.is_empty();

    view! {
        <span class=cls title=name.clone() aria-label=name>
            {if has_src {
                view! { <img src=src alt=alt draggable="false" /> }.into_any()
            } else {
                text.into_any()
            }}
            {status.map(|s| view! {
                <span class=format!("c-avatar__status {}", s.class())></span>
            })}
        </span>
    }
}
