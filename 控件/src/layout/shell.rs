use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::base::overlay::OverlayHost;
use crate::widgets::toast::ToastHost;

/// 应用外壳。`.l-app` 负责圆角、外影与两栏布局。
#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    // 浮层栈在根上 provide 一次：所有页面的 Dialog/Select 共用一条栈，
    // 否则各自 use_or_provide 会拿到互不相见的两个 host。不产生 DOM。
    let _host = OverlayHost::use_or_provide();
    view! { <div class="l-app">{children()}<ToastHost /></div> }
}

/// 主内容列（顶栏之下）。
#[component]
pub fn Main(children: Children) -> impl IntoView {
    view! { <div class="l-main">{children()}</div> }
}

/// 滚动舞台。内容超出一屏时在这里滚。
#[component]
pub fn ScrollArea(children: Children) -> impl IntoView {
    view! { <div class="l-stage">{children()}</div> }
}

/// 侧边导航栏（窗格材质，不是玻璃）。
#[component]
pub fn NavColumn(children: Children) -> impl IntoView {
    view! {
        <aside class="l-nav c-pane" aria-label="导航栏">
            <div class="l-nav__head"></div>
            <div class="l-nav__body">{children()}</div>
            <div class="c-topveil c-topveil--nav" data-on="0"></div>
        </aside>
    }
}

#[component]
pub fn NavGroup(children: Children) -> impl IntoView {
    view! { <div class="c-nav" role="listbox">{children()}</div> }
}

#[component]
pub fn NavSection(#[prop(into, optional)] label: String) -> impl IntoView {
    view! { <div class="c-nav__section" role="presentation">{label}</div> }
}

#[component]
pub fn NavItem(
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] selected: MaybeProp<bool>,
    #[prop(optional)] count: Option<u32>,
    #[prop(into, optional)] badge: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    let aria_selected = move || {
        if selected.get().unwrap_or(false) {
            "true"
        } else {
            "false"
        }
    };
    let title = label.clone();

    view! {
        <div
            class="c-nav__item"
            role="option"
            aria-selected=aria_selected
            aria-disabled=disabled.then_some("true")
            tabindex=if disabled { "-1" } else { "0" }
            title=title
            on:click=move |_| {
                if disabled {
                    return;
                }
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if disabled {
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
                <span class="c-nav__icon"><Icon name=name size=16 /></span>
            })}
            <span class="c-nav__label">{label}</span>
            {count.map(|n| view! { <span class="c-nav__count">{n}</span> })}
            {(!badge.is_empty()).then(|| view! { <span class="c-nav__badge">{badge}</span> })}
        </div>
    }
}
