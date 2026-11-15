use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), disabled: false }
    }
}

#[component]
pub fn Select(
    #[prop(into)] options: Vec<SelectOption>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] width: u32,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let trigger_ref = NodeRef::<html::Button>::new();
    let menu_pos = RwSignal::new((0.0f64, 0.0f64, 0.0f64));
    let width = if width == 0 { 180 } else { width };
    let options_for_menu = options.clone();
    let options_for_label = options.clone();
    let current_label = move || {
        let v = value.get().unwrap_or_default();
        options_for_label.iter().find(|o| o.value == v).map(|o| o.label.clone()).unwrap_or_default()
    };
    let has_value = move || !value.get().unwrap_or_default().is_empty();

    let menu = move || {
        options_for_menu.iter().map(|opt| {
            let opt_value = opt.value.clone();
            let opt_label = opt.label.clone();
            let disabled = opt.disabled;
            let v1 = opt_value.clone();
            let v2 = opt_value.clone();
            view! {
                <div
                    class="c-menu__item"
                    role="menuitemradio"
                    tabindex="-1"
                    data-label=opt_label.clone()
                    aria-checked=move || (value.get().as_deref() == Some(v1.as_str())).to_string()
                    aria-disabled=disabled.then_some("true")
                    on:click=move |_| {
                        if disabled {
                            return;
                        }
                        if let Some(cb) = on_change {
                            cb.run(opt_value.clone());
                        }
                        open.set(false);
                    }
                >
                    <span class="c-menu__check" data-role="menuitemradio">
                        {move || (value.get().as_deref() == Some(v2.as_str())).then(|| view! { <Icon name=IconName::CHECK size=12 /> })}
                    </span>
                    <span class="c-menu__label">{opt_label.clone()}</span>
                </div>
            }
        }).collect::<Vec<_>>()
    };

    view! {
        <div class="c-select" data-open=move || open.get().then_some("1") style=format!("position:relative;width:{width}px")>
            <button
                class="c-btn"
                type="button"
                aria-haspopup="menu"
                aria-label=label
                aria-expanded=move || open.get().to_string()
                style="width:100%;justify-content:space-between"
                node_ref=trigger_ref
                on:click=move |_| {
                    let next = !open.get_untracked();
                    if next {
                        if let Some(el) = trigger_ref.get() {
                            let r = el.get_bounding_client_rect();
                            menu_pos.set((r.left(), r.bottom() + 6.0, r.width()));
                        }
                    }
                    open.set(next);
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        open.set(false);
                    } else if ev.key() == "ArrowDown" || ev.key() == "ArrowUp" {
                        ev.prevent_default();
                        open.set(true);
                    }
                }
            >
                <span class="l-fill" style="text-align:left">
                    {move || if has_value() { current_label() } else { "请选择".to_string() }}
                </span>
                <span style="display:grid" style:transform=move || format!("rotate({}deg)", if open.get() { 180 } else { 0 })>
                    <Icon name=IconName::CHEVRON_DOWN size=12 />
                </span>
            </button>

            <div
                style=move || if open.get() { "position:fixed;inset:0;z-index:899" } else { "display:none" }
                on:click=move |_| open.set(false)
            ></div>
            <div
                class="c-menu"
                role="menu"
                tabindex="-1"
                style=move || if open.get() {
                    let (l, t, w) = menu_pos.get();
                    format!("position:fixed;left:{l}px;top:{t}px;width:{w}px;z-index:900;min-width:0")
                } else {
                    "display:none".to_string()
                }
            >
                {menu}
            </div>
        </div>
    }
}
