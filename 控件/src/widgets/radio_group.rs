use leptos::html;
use leptos::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RadioOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl RadioOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), disabled: false }
    }
}

#[component]
pub fn RadioGroup(
    #[prop(into)] options: Vec<RadioOption>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] inline: bool,
) -> impl IntoView {
    let refs: Vec<NodeRef<html::Div>> = (0..options.len()).map(|_| NodeRef::new()).collect();
    let all = options.clone();

    let rows = options.into_iter().enumerate().map(|(i, opt)| {
        let value_state = opt.value.clone();
        let value_tab = opt.value.clone();
        let value_click = opt.value.clone();
        let row_ref = refs[i];
        let all = all.clone();
        let on_change = on_change;
        view! {
            <div
                class="c-radio-row"
                role="radio"
                data-value=opt.value.clone()
                aria-checked=move || if value.get().as_deref() == Some(value_state.as_str()) { "true" } else { "false" }
                tabindex=move || if value.get().as_deref() == Some(value_tab.as_str()) { "0" } else { "-1" }
                aria-disabled=opt.disabled.then_some("true")
                node_ref=row_ref
                on:click=move |_| {
                    if opt.disabled {
                        return;
                    }
                    if let Some(cb) = on_change {
                        cb.run(value_click.clone());
                    }
                }
                on:keydown={
                    let all = all.clone();
                    let current = opt.value.clone();
                    let refs = refs.clone();
                    move |ev: web_sys::KeyboardEvent| {
                        let step = match ev.key().as_str() {
                            "ArrowRight" | "ArrowDown" => 1isize,
                            "ArrowLeft" | "ArrowUp" => -1,
                            _ => return,
                        };
                        ev.prevent_default();
                        let Some(mut idx) = all.iter().position(|o| o.value == current) else { return };
                        for _ in 0..all.len() {
                            idx = (idx as isize + step).rem_euclid(all.len() as isize) as usize;
                            if !all[idx].disabled {
                                break;
                            }
                        }
                        if let Some(el) = refs[idx].get() {
                            let _ = el.focus();
                        }
                        if let Some(cb) = on_change {
                            cb.run(all[idx].value.clone());
                        }
                    }
                }
            >
                <span class="c-radio" aria-hidden="true"></span>
                <span style="font-size:var(--fs-13)">{opt.label}</span>
            </div>
        }
    }).collect::<Vec<_>>();

    let cls = if inline { "c-radio-group c-radio-group--inline" } else { "c-radio-group" };
    view! {
        <div class=cls role="radiogroup">{rows}</div>
    }
}
