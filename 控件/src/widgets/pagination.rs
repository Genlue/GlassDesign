use leptos::prelude::*;

use crate::base::i18n::{fill, t};
use crate::base::icon::{Icon, IconName};

fn window_pages(current: u32, total: u32) -> Vec<Option<u32>> {
    if total <= 7 {
        return (1..=total).map(Some).collect();
    }
    let mut set = std::collections::BTreeSet::new();
    set.insert(1);
    set.insert(total);
    for d in -1i32..=1 {
        let p = current as i32 + d;
        if p >= 1 && p <= total as i32 {
            set.insert(p as u32);
        }
    }
    let mut out = Vec::new();
    let mut prev = 0;
    for p in set {
        if p > prev + 1 {
            out.push(None);
        }
        out.push(Some(p));
        prev = p;
    }
    out
}

#[component]
pub fn Pagination(
    #[prop(into, optional)] page: MaybeProp<u32>,
    #[prop(into, optional)] total_pages: MaybeProp<u32>,
    #[prop(optional)] on_change: Option<Callback<u32>>,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let total = move || total_pages.get().unwrap_or(1).max(1);
    let current = move || page.get().unwrap_or(1).clamp(1, total());
    let go = move |p: u32| {
        let p = p.clamp(1, total());
        if p != current() {
            if let Some(cb) = on_change {
                cb.run(p);
            }
        }
    };

    let pages = move || {
        window_pages(current(), total())
            .into_iter()
            .map(|p| match p {
                Some(n) => view! {
                    <button class="c-pagination__btn c-pagination__page" type="button"
                        data-page=n
                        aria-label=fill(t().page_label_fmt, &[("n", &n.to_string())])
                        aria-current=(n == current()).then_some("page")
                        on:click=move |_| go(n)>{n}</button>
                }
                .into_any(),
                None => view! { <span class="c-pagination__gap" aria-hidden="true">"…"</span> }
                    .into_any(),
            })
            .collect::<Vec<_>>()
    };

    // 兜底查文案表，留在属性闭包里（切语言要跟着变）。
    let label_store = StoredValue::new_local(label);
    let label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().pagination_label.to_string()
        } else {
            own
        }
    };
    view! {
        <nav class="c-pagination" role="navigation" aria-label=label>
            <div class="c-pagination__pager" role="group" aria-label=move || t().pages_label.to_string()>
                <button class="c-pagination__btn c-pagination__nav" type="button"
                    aria-label=move || t().prev_page.to_string() title=move || t().prev_page.to_string()
                    aria-disabled=move || (current() <= 1).then_some("true")
                    on:click=move |_| go(current().saturating_sub(1))>
                    <Icon name=IconName::CHEVRON_LEFT size=14 />
                </button>
                <div class="c-pagination__pages">{pages}</div>
                <button class="c-pagination__btn c-pagination__nav" type="button"
                    aria-label=move || t().next_page.to_string() title=move || t().next_page.to_string()
                    aria-disabled=move || (current() >= total()).then_some("true")
                    on:click=move |_| go(current() + 1)>
                    <Icon name=IconName::CHEVRON_RIGHT size=14 />
                </button>
            </div>
            <div class="c-pagination__meta">
                <span class="c-pagination__summary" aria-live="polite" aria-atomic="true">
                    {move || fill(
                        t().page_status_fmt,
                        &[("page", &current().to_string()), ("total", &total().to_string())],
                    )}
                </span>
            </div>
        </nav>
    }
}
