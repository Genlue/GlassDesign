use leptos::prelude::*;

#[component]
pub fn FormField(
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] hint: String,
    #[prop(optional)] required: bool,
    #[prop(into, optional)] error: MaybeProp<String>,
    #[prop(optional)] inline: bool,
    children: Children,
    #[prop(optional)] actions: Option<Children>,
) -> impl IntoView {
    let err = move || error.get().unwrap_or_default();
    let has_error = move || !err().is_empty();
    let cls = if inline {
        "c-formfield c-formfield--inline"
    } else {
        "c-formfield"
    };

    view! {
        <div class=cls data-invalid=move || has_error().then_some("1")
            style="--ff-label-w:96px">
            <div class="c-formfield__head">
                {(!label.is_empty()).then(|| view! {
                    <span class="c-formfield__label">
                        {label.clone()}
                        {required.then(|| view! { <span class="c-formfield__req" aria-hidden="true">"*"</span> })}
                    </span>
                })}
                {actions.map(|a| view! { <div class="c-formfield__actions">{a()}</div> })}
            </div>
            <div class="c-formfield__body">{children()}</div>
            <div class="c-formfield__foot">
                {(!hint.is_empty()).then(|| view! { <div class="c-formfield__hint">{hint.clone()}</div> })}
                {move || has_error().then(|| view! {
                    <div class="c-formfield__error" role="alert">{err()}</div>
                })}
            </div>
        </div>
    }
}
