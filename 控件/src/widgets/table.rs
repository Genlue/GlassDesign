use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableColumn {
    pub key: String,
    pub label: String,
    pub align_right: bool,
    pub sortable: bool,
}

impl TableColumn {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            align_right: false,
            sortable: false,
        }
    }
    pub fn right(mut self) -> Self {
        self.align_right = true;
        self
    }
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableRow {
    pub key: String,
    pub cells: Vec<String>,
}

impl TableRow {
    pub fn new(key: impl Into<String>, cells: Vec<impl Into<String>>) -> Self {
        Self {
            key: key.into(),
            cells: cells.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableSort {
    pub key: String,
    pub dir: SortDir,
}

#[component]
pub fn Table(
    #[prop(into)] columns: Vec<TableColumn>,
    #[prop(into)] rows: Vec<TableRow>,
    #[prop(into, optional)] sort: MaybeProp<TableSort>,
    #[prop(optional)] on_sort: Option<Callback<(String, SortDir)>>,
    #[prop(into, optional)] selected_key: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] dense: bool,
) -> impl IntoView {
    let cls = if dense {
        "c-table c-table--dense"
    } else {
        "c-table"
    };
    let selected = selected_key;

    let head = columns.iter().map(|col| {
        let key = col.key.clone();
        let label = col.label.clone();
        let aria = sort.get().filter(|s| s.key == col.key).map(|s| match s.dir {
            SortDir::Asc => "ascending",
            SortDir::Desc => "descending",
        });
        let sort_for_click = sort.get();
        let on_sort = on_sort;
        let sortable = col.sortable;
        let align = col.align_right.then_some("text-align:right");
        view! {
            <th style=align aria-sort=aria>
                {if sortable {
                    let next = match sort_for_click.as_ref().filter(|s| s.key == key) {
                        Some(TableSort { dir: SortDir::Asc, .. }) => SortDir::Desc,
                        _ => SortDir::Asc,
                    };
                    let key2 = key.clone();
                    view! {
                        <button class="c-table__sort" type="button"
                            on:click=move |_| {
                                if let Some(cb) = on_sort { cb.run((key2.clone(), next)); }
                            }>
                            {label}
                            <Icon
                                name=if matches!(next, SortDir::Asc) { IconName::CHEVRON_UP } else { IconName::CHEVRON_DOWN }
                                size=11
                            />
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <span>{label}</span>
                    }.into_any()
                }}
            </th>
        }
    }).collect::<Vec<_>>();

    let body = rows
        .iter()
        .map(|row| {
            let key = row.key.clone();
            let is_selected = selected.get().as_deref() == Some(row.key.as_str());
            let cells = row.cells.clone();
            let on_select = on_select;
            let key_for_click = key.clone();
            view! {
                <tr
                    aria-selected=(on_select.is_some()).then(|| is_selected.to_string())
                    on:click=move |_| {
                        if let Some(cb) = on_select { cb.run(key_for_click.clone()); }
                    }
                >
                    {cells.into_iter().enumerate().map(|(i, text)| {
                        let right = columns.get(i).map(|c| c.align_right).unwrap_or(false);
                        view! {
                            <td class=right.then_some("c-table__num")>{text}</td>
                        }
                    }).collect::<Vec<_>>()}
                </tr>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <table class=cls>
            <thead><tr>{head}</tr></thead>
            <tbody>{body}</tbody>
        </table>
    }
}
