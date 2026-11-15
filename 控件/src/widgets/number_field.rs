use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[component]
pub fn NumberField(
    #[prop(into, optional)] value: MaybeProp<f64>,
    #[prop(optional)] min: f64,
    #[prop(optional)] max: f64,
    #[prop(optional)] step: f64,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<f64>>,
) -> impl IntoView {
    let max = if max <= min { min + 1_000_000.0 } else { max };
    let step = if step <= 0.0 { 1.0 } else { step };
    let input_ref = NodeRef::<html::Input>::new();
    let current = move || value.get().unwrap_or(min).clamp(min, max);

    let commit = move |next: f64| {
        let q = (min + ((next - min) / step).round() * step).clamp(min, max);
        if (q - current()).abs() > f64::EPSILON {
            if let Some(cb) = on_change {
                cb.run((q * 1_000_000.0).round() / 1_000_000.0);
            }
        }
    };
    let parse_commit = move |text: &str| {
        if let Ok(v) = text.trim().replace(',', "").parse::<f64>() {
            commit(v);
        } else {
            if let Some(el) = input_ref.get() {
                el.set_value(&format!("{}", current()));
            }
        }
    };

    view! {
        <div class="c-numberfield" data-disabled=disabled.then_some("1")>
            <input
                class="c-field__input c-numberfield__input selectable"
                type="text"
                inputmode="decimal"
                role="spinbutton"
                autocomplete="off"
                aria-label=label
                placeholder=placeholder
                disabled=disabled
                aria-valuenow=move || current().to_string()
                aria-valuemin=min.to_string()
                aria-valuemax=max.to_string()
                value=move || format!("{}", current())
                prop:value=move || format!("{}", current())
                node_ref=input_ref
                on:change=move |ev| parse_commit(&event_target_value(&ev))
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    let next = match ev.key().as_str() {
                        "ArrowUp" => current() + step,
                        "ArrowDown" => current() - step,
                        "PageUp" => current() + step * 10.0,
                        "PageDown" => current() - step * 10.0,
                        "Home" => min,
                        "End" => max,
                        "Enter" => { parse_commit(&event_target_value(&ev)); return; }
                        _ => return,
                    };
                    ev.prevent_default();
                    commit(next);
                }
            />
            <span class="c-numberfield__spin">
                <button class="c-numberfield__btn" type="button" tabindex="-1" aria-label="增加"
                    disabled=disabled
                    on:click=move |_| commit(current() + step)>
                    <Icon name=IconName::CHEVRON_UP size=9 />
                </button>
                <button class="c-numberfield__btn" type="button" tabindex="-1" aria-label="减少"
                    disabled=disabled
                    on:click=move |_| commit(current() - step)>
                    <Icon name=IconName::CHEVRON_DOWN size=9 />
                </button>
            </span>
        </div>
    }
}
