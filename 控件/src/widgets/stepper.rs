use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum StepperOrientation {
    #[default]
    Vertical,
    Horizontal,
}

#[component]
pub fn Stepper(
    #[prop(into, optional)] value: MaybeProp<f64>,
    #[prop(optional)] min: f64,
    #[prop(optional)] max: f64,
    #[prop(optional)] step: f64,
    #[prop(optional)] orientation: StepperOrientation,
    #[prop(into, optional)] label: String,
    #[prop(optional)] on_change: Option<Callback<f64>>,
) -> impl IntoView {
    let min = if min == 0.0 && max == 0.0 { 0.0 } else { min };
    let max = if max == 0.0 { 100.0 } else { max };
    let step = if step == 0.0 { 1.0 } else { step };
    let current = move || value.get().unwrap_or(min).clamp(min, max);
    let at_min = move || current() <= min;
    let at_max = move || current() >= max;
    let bump = move |delta: f64| {
        let next = (current() + delta).clamp(min, max);
        if (next - current()).abs() > f64::EPSILON {
            if let Some(cb) = on_change {
                cb.run((next * 1_000_000.0).round() / 1_000_000.0);
            }
        }
    };

    if orientation == StepperOrientation::Horizontal {
        view! {
            <div class="c-stepper c-stepper--h" role="group" aria-label=label>
                <button class="c-stepper__btn" type="button" aria-label="减少"
                    disabled=move || at_min()
                    on:click=move |_| bump(-step)>
                    <Icon name=IconName::MINUS size=12 />
                </button>
                <span class="c-stepper__value">{move || current().to_string()}</span>
                <button class="c-stepper__btn" type="button" aria-label="增加"
                    disabled=move || at_max()
                    on:click=move |_| bump(step)>
                    <Icon name=IconName::PLUS size=12 />
                </button>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="c-stepper" role="group" aria-label=label>
                <button class="c-stepper__btn" type="button" aria-label="增加"
                    disabled=move || at_max()
                    on:click=move |_| bump(step)>
                    <Icon name=IconName::CHEVRON_UP size=10 />
                </button>
                <button class="c-stepper__btn" type="button" aria-label="减少"
                    disabled=move || at_min()
                    on:click=move |_| bump(-step)>
                    <Icon name=IconName::CHEVRON_DOWN size=10 />
                </button>
            </div>
        }
        .into_any()
    }
}
