use leptos::prelude::*;

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
    #[prop(optional)] total_pages: u32,
    #[prop(optional)] on_change: Option<Callback<u32>>,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let total = total_pages.max(1);
    let current = move || page.get().unwrap_or(1).clamp(1, total);
    let go = move |p: u32| {
        let p = p.clamp(1, total);
        if p != current() {
            if let Some(cb) = on_change { cb.run(p); }
        }
    };

    let pages = move || window_pages(current(), total).into_iter().map(|p| {
        match p {
            Some(n) => view! {
                <button class="c-pagination__btn c-pagination__page" type="button"
                    data-page=n
                    aria-label=format!("第 {n} 页")
                    aria-current=(n == current()).then_some("page")
                    on:click=move |_| go(n)>{n}</button>
            }.into_any(),
            None => view! { <span class="c-pagination__gap" aria-hidden="true">"…"</span> }.into_any(),
        }
    }).collect::<Vec<_>>();

    let label = if label.is_empty() { "分页".to_string() } else { label };
    view! {
        <nav class="c-pagination" role="navigation" aria-label=label>
            <div class="c-pagination__pager" role="group" aria-label="页码">
                <button class="c-pagination__btn c-pagination__nav" type="button"
                    aria-label="上一页" title="上一页"
                    aria-disabled=move || (current() <= 1).then_some("true")
                    on:click=move |_| go(current().saturating_sub(1))>
                    <Icon name=IconName::CHEVRON_LEFT size=14 />
                </button>
                <div class="c-pagination__pages">{pages}</div>
                <button class="c-pagination__btn c-pagination__nav" type="button"
                    aria-label="下一页" title="下一页"
                    aria-disabled=move || (current() >= total).then_some("true")
                    on:click=move |_| go(current() + 1)>
                    <Icon name=IconName::CHEVRON_RIGHT size=14 />
                </button>
            </div>
            <div class="c-pagination__meta">
                <span class="c-pagination__summary" aria-live="polite" aria-atomic="true">
                    {move || format!("第 {} 页，共 {} 页", current(), total)}
                </span>
            </div>
        </nav>
    }
}
