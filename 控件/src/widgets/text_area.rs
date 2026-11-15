use leptos::prelude::*;

#[component]
pub fn TextArea(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] rows: u32,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] disabled: MaybeProp<bool>,
    #[prop(into, optional)] readonly: MaybeProp<bool>,
    #[prop(into, optional)] invalid: MaybeProp<bool>,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(optional)] on_enter: Option<Callback<String>>,
) -> impl IntoView {
    let rows = if rows == 0 { 4 } else { rows };
    let invalid_now = move || invalid.get().unwrap_or(false);

    view! {
        <div data-invalid=move || invalid_now().then_some("1")>
            <textarea
                class="c-textarea selectable"
                rows=rows
                placeholder=placeholder
                aria-label=label
                aria-invalid=move || invalid_now().to_string()
                disabled=move || disabled.get().unwrap_or(false)
                readonly=move || readonly.get().unwrap_or(false)
                prop:value=move || value.get().unwrap_or_default()
                on:input=move |ev| {
                    if let Some(cb) = on_input { cb.run(event_target_value(&ev)); }
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    let cmd = ev.meta_key() || ev.ctrl_key();
                    if !cmd || ev.key() != "Enter" || ev.is_composing() || ev.key_code() == 229 {
                        return;
                    }
                    ev.prevent_default();
                    if let Some(cb) = on_enter { cb.run(event_target_value(&ev)); }
                }
            ></textarea>
        </div>
    }
}
