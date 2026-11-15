//! 数据表：列宽 / 列序 / 排序 / 选择 / 虚拟行。
//!
//! 与 `table.rs` 的分界沿用旧库：`Table` 是「读」（一次渲染，不碰状态），
//! `DataTable` 是「改」。两者合在一起会让只读表格背上一堆用不到的分支。
//!
//! 几条刻意的取舍：
//! 1) 列宽、列序、排序都只存在组件内部 —— 调用方传进来的 `columns` / `rows`
//!    是数据，不是控件状态。拖动一次列宽不该改写调用方的常量。
//! 2) 选中集用稳定 `id`，排序 / 虚拟滚动都不会丢选中。
//! 3) 虚拟滚动用「上下两个撑高行」，而不是绝对定位：table 的布局引擎本来
//!    就按行高排队，绝对定位还得手算每行 top，并和 sticky 表头对账。
//! 4) 行内编辑不做（`__editor` / `[data-editing]` 类留给未来），因为编辑
//!    涉及输入法、校验、提交协议，和「展示 + 选择」是两条独立的状态机。

use std::cmp::Ordering;
use std::collections::HashMap;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::widgets::progress::Skeleton;
use crate::widgets::selection::{SelectionMode, SelectionState};
use crate::widgets::table::SortDir;

/// 列对齐。写成枚举而不是 `bool align_right`，是因为中间对齐迟早要用。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ColAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl ColAlign {
    pub fn as_str(self) -> &'static str {
        match self {
            ColAlign::Left => "left",
            ColAlign::Center => "center",
            ColAlign::Right => "right",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DataColumn {
    pub key: String,
    pub title: String,
    /// 初始宽度（px）。<= 0 时回落到 160。
    pub width: f64,
    /// 拖窄时的下限（px）。
    pub min_width: f64,
    pub align: ColAlign,
    pub sortable: bool,
}

impl DataColumn {
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            width: 160.0,
            min_width: 56.0,
            align: ColAlign::Left,
            sortable: false,
        }
    }

    pub fn width(mut self, px: f64) -> Self {
        self.width = px;
        self
    }

    pub fn min_width(mut self, px: f64) -> Self {
        self.min_width = px;
        self
    }

    pub fn align(mut self, align: ColAlign) -> Self {
        self.align = align;
        self
    }

    pub fn right(self) -> Self {
        self.align(ColAlign::Right)
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DataTableRow {
    pub id: String,
    pub cells: Vec<String>,
}

impl DataTableRow {
    pub fn new(id: impl Into<String>, cells: Vec<impl Into<String>>) -> Self {
        Self {
            id: id.into(),
            cells: cells.into_iter().map(Into::into).collect(),
        }
    }
}

/// 默认宽度。与旧库一致，避免调用方不写宽度时列被内容反推。
const DEFAULT_COL_WIDTH: f64 = 160.0;
/// 虚拟窗口上下各多渲染几行：抵消快速滚动时的一帧空白。
const OVERSCAN: usize = 6;

fn normalize_width(col: &DataColumn) -> f64 {
    let min = if col.min_width > 0.0 { col.min_width } else { 1.0 };
    if col.width > 0.0 {
        col.width.max(min)
    } else {
        DEFAULT_COL_WIDTH.max(min)
    }
}

/// 排序比较：空值沉底；两边都能 parse 成 f64 时按数值，否则按字符串。
/// 不做 locale 排序 —— 那需要额外依赖，而且「纯字符串比较」是既定契约。
fn cmp_cell(a: &str, b: &str) -> Ordering {
    let ea = a.is_empty();
    let eb = b.is_empty();
    if ea || eb {
        return ea.cmp(&eb);
    }
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x
            .partial_cmp(&y)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.cmp(b)),
        _ => a.cmp(b),
    }
}

/// 指针落在哪个插入缝：左半边插在本列前，右半边插在后。
fn drop_index(root: &web_sys::Element, x: f64) -> Option<usize> {
    let list = root.query_selector_all(".c-datatable__th").ok()?;
    let mut insert = 0usize;
    for i in 0..list.length() {
        let Some(node) = list.get(i) else { continue };
        let Ok(el) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let r = el.get_bounding_client_rect();
        if x < r.left() {
            return Some(i as usize);
        }
        if x <= r.right() {
            return Some(if x < r.left() + r.width() / 2.0 {
                i as usize
            } else {
                i as usize + 1
            });
        }
        insert = i as usize + 1;
    }
    Some(insert)
}

/// 把 `from` 列搬到插入点 `insert`（插入点按「搬走之前」的序列计算）。
fn reorder_columns(order: RwSignal<Vec<String>>, from: usize, insert: usize) {
    let n = order.get_untracked().len();
    if from >= n || insert > n {
        return;
    }
    // 拖到自己身上 / 自己右边那条缝：没有变化。
    if insert == from || insert == from + 1 {
        return;
    }
    order.update(|o| {
        let key = o.remove(from);
        let at = (if insert > from { insert - 1 } else { insert }).min(o.len());
        o.insert(at, key);
    });
}

#[component]
pub fn DataTable(
    #[prop(into)] columns: Vec<DataColumn>,
    #[prop(into)] rows: Vec<DataTableRow>,
    #[prop(optional)] selection: SelectionMode,
    #[prop(into, optional)] default_selected: Vec<String>,
    #[prop(optional)] on_selection_change: Option<Callback<Vec<String>>>,
    /// 开启虚拟行时必须给固定行高；CSS 的 `--dt-row-h` 与窗口计算共用它。
    #[prop(optional)] virtual_row_height: Option<f64>,
    #[prop(optional)] loading: bool,
    #[prop(default = 6)] loading_rows: usize,
    #[prop(into, optional)] empty_text: String,
    #[prop(into, optional)] label: String,
    #[prop(default = true)] resizable: bool,
) -> impl IntoView {
    let label = if label.is_empty() {
        "数据表".to_string()
    } else {
        label
    };
    let empty_text = if empty_text.is_empty() {
        "没有数据".to_string()
    } else {
        empty_text
    };
    let row_h = virtual_row_height.unwrap_or(28.0).max(1.0);
    let virtual_mode = virtual_row_height.is_some();

    let root_ref = NodeRef::<html::Div>::new();
    let scroll_ref = NodeRef::<html::Div>::new();

    // 列索引只用于「列 key → 原始 cells 下标」，排序 / 拖列都以 key 为准。
    let col_index: HashMap<String, usize> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.key.clone(), i))
        .collect();
    let col_map: HashMap<String, DataColumn> = columns
        .iter()
        .cloned()
        .map(|c| (c.key.clone(), c))
        .collect();

    let order = RwSignal::new(columns.iter().map(|c| c.key.clone()).collect::<Vec<_>>());
    let widths = RwSignal::new(
        columns
            .iter()
            .map(|c| (c.key.clone(), normalize_width(c)))
            .collect::<HashMap<String, f64>>(),
    );
    let sort = RwSignal::new(None::<(String, SortDir)>);

    let sel = RwSignal::new(SelectionState::with_selected(default_selected));
    let row_count = rows.len();

    let ids_rows = rows.clone();
    let ids_cols = columns.clone();
    let view_ids = Memo::new(move |_| {
        // 排序后的稳定 ID 序列：Shift 连选必须按「当前看到」的顺序走。
        sorted_rows(&ids_rows, &ids_cols, sort.get())
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
    });

    /* ---------------------------------------------------------- 虚拟窗口 */

    let win = RwSignal::new((0usize, if virtual_mode { 18 } else { row_count }));

    let clamp_win = move || {
        let n = row_count;
        let (s, e) = win.get();
        let s = s.min(n);
        (s, e.max(s).min(n))
    };

    let win_rows = rows.clone();
    let win_cols = columns.clone();
    let window_rows = Memo::new(move |_| {
        if !virtual_mode {
            return sorted_rows(&win_rows, &win_cols, sort.get());
        }
        let all = sorted_rows(&win_rows, &win_cols, sort.get());
        let (s, e) = clamp_win();
        all[s..e].to_vec()
    });

    let on_scroll = move |_| {
        if !virtual_mode {
            return;
        }
        let Some(el) = scroll_ref.get_untracked() else {
            return;
        };
        let top = el.scroll_top() as f64;
        let view_h = el.client_height() as f64;
        let start = ((top / row_h).floor() as isize - OVERSCAN as isize).max(0) as usize;
        let end = (((top + view_h) / row_h).ceil() as usize + OVERSCAN).min(row_count);
        let end = end.max(start);
        if win.get_untracked() != (start, end) {
            win.set((start, end));
        }
    };

    /* ------------------------------------------------------------ 列宽拖拽 */

    let resizing_key = RwSignal::new(None::<String>);
    let resize_start = RwSignal::new((0.0f64, 0.0f64));

    /* ------------------------------------------------------------ 列拖拽 */

    let drag_from = RwSignal::new(None::<usize>);
    let drag_start = RwSignal::new((0.0f64, 0.0f64));
    let drag_active = RwSignal::new(false);
    let drop_at = RwSignal::new(None::<usize>);
    // 拖完列后浏览器还会补一个 click；这个开关把那次点击吃掉，避免误触排序。
    let suppress_click = RwSignal::new(false);

    let move_handle = window_event_listener(ev::pointermove, move |ev: web_sys::PointerEvent| {
        let Some(from) = drag_from.get_untracked() else {
            return;
        };
        if !drag_active.get_untracked() {
            let (sx, sy) = drag_start.get_untracked();
            if (ev.client_x() as f64 - sx).abs() < 5.0
                && (ev.client_y() as f64 - sy).abs() < 5.0
            {
                return;
            }
            drag_active.set(true);
        }
        let _ = from;
        if let Some(root) = root_ref.get_untracked() {
            drop_at.set(drop_index(&root, ev.client_x() as f64));
        }
    });

    let up_handle = window_event_listener(ev::pointerup, move |_| {
        let Some(from) = drag_from.get_untracked() else {
            return;
        };
        drag_from.set(None);
        if !drag_active.get_untracked() {
            drop_at.set(None);
            return;
        }
        drag_active.set(false);
        suppress_click.set(true);
        if let Some(at) = drop_at.get_untracked() {
            reorder_columns(order, from, at);
        }
        drop_at.set(None);
        // 指针捕获失败（例如合成事件）时，手柄自己的 pointerup 收不到；
        // 这里兜底清掉，避免整表一直停在 col-resize 光标上。
        resizing_key.set(None);
    });

    let cancel_handle = window_event_listener(ev::pointercancel, move |_| {
        drag_from.set(None);
        drag_active.set(false);
        drop_at.set(None);
        resizing_key.set(None);
    });

    on_cleanup(move || {
        move_handle.remove();
        up_handle.remove();
        cancel_handle.remove();
    });

    /* --------------------------------------------------------------- 表头 */

    let header = {
        let col_map = col_map.clone();
        move || {
            let col_map = col_map.clone();
            view! {
                <For
                    each=move || order.get()
                    key=|k| k.clone()
                    children=move |key: String| {
                        let col = col_map
                            .get(&key)
                            .cloned()
                            .unwrap_or_else(|| DataColumn::new(key.clone(), key.clone()));
                        header_cell(
                            key,
                            col,
                            root_ref,
                            order,
                            widths,
                            sort,
                            resizable,
                            drag_from,
                            drag_start,
                            drag_active,
                            drop_at,
                            suppress_click,
                            resizing_key,
                            resize_start,
                        )
                    }
                />
            }
        }
    };

    /* --------------------------------------------------------------- 行 */

    let row_view = {
        let col_index = col_index.clone();
        let columns = columns.clone();
        let mode = selection;
        let sel = sel;
        let order_sig = order;
        move |row: DataTableRow| -> AnyView {
            let id_click = row.id.clone();
            let id_aria = row.id.clone();
            let id_key = row.id.clone();
            let cells_row = row.clone();
            let cols = columns.clone();
            let idx_map = col_index.clone();
            view! {
                <tr
                    class="c-datatable__row"
                    role="row"
                    data-key=id_key
                    aria-selected=move || {
                        mode.is_selectable()
                            .then(|| sel.get().contains(&id_aria).to_string())
                    }
                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                        if ev.button() != 0 || !mode.is_selectable() {
                            return;
                        }
                        // 行里以后可能放按钮 / 链接（编辑、操作列），那些区域不归选择管。
                        if let Some(target) = ev.target() {
                            if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                                if el
                                    .closest("button, a, input, textarea, select")
                                    .ok()
                                    .flatten()
                                    .is_some()
                                {
                                    return;
                                }
                            }
                        }
                        let order_now = order_sig.get_untracked();
                        let mut next = sel.get_untracked();
                        let changed = next.click(
                            mode,
                            &id_click,
                            &order_now,
                            ev.ctrl_key() || ev.meta_key(),
                            ev.shift_key(),
                        );
                        if changed {
                            sel.set(next);
                            if let Some(cb) = on_selection_change {
                                cb.run(sel.get_untracked().ordered(&order_now));
                            }
                        }
                    }
                >
                    <For
                        each=move || order_sig.get()
                        key=|k| k.clone()
                        children=move |key: String| {
                            let ci = idx_map.get(&key).copied().unwrap_or(usize::MAX);
                            let align = cols.get(ci).map(|c| c.align).unwrap_or(ColAlign::Left);
                            let text = cells_row.cells.get(ci).cloned().unwrap_or_default();
                            view! {
                                <td
                                    class="c-datatable__cell"
                                    role="gridcell"
                                    data-align=align.as_str()
                                >
                                    {text}
                                </td>
                            }
                            .into_any()
                        }
                    />
                    <td class="c-datatable__filler" role="presentation" aria-hidden="true"></td>
                </tr>
            }
            .into_any()
        }
    };

    let row_count = rows.len();
    let loading_rows = loading_rows.max(1);
    let loading_view = move || {
        (0..loading_rows)
            .map(|_| {
                view! {
                    <tr class="c-datatable__row" aria-hidden="true">
                        <td class="c-datatable__cell" colspan=move || (order.get().len() + 1).to_string()>
                            <Skeleton width="70%" />
                        </td>
                    </tr>
                }
            })
            .collect_view()
    };

    let root_style = format!("--dt-row-h:{row_h}px");

    view! {
        <div
            class="c-datatable"
            role="grid"
            aria-label=label
            aria-colcount=columns.len().to_string()
            aria-rowcount=move || (row_count + 1).to_string()
            aria-multiselectable=selection.is_multiple().then_some("true")
            data-resizing=move || resizing_key.get().is_some().then_some("1")
            data-drag=move || drag_active.get().then_some("1")
            style=root_style
            node_ref=root_ref
        >
            <div class="c-datatable__scroll" node_ref=scroll_ref on:scroll=on_scroll>
                <table class="c-datatable__table" role="presentation">
                    <colgroup>
                        <For
                            each=move || order.get()
                            key=|k| k.clone()
                            children=move |key: String| {
                                view! {
                                    <col style=move || {
                                        let w = widths
                                            .get()
                                            .get(&key)
                                            .copied()
                                            .unwrap_or(DEFAULT_COL_WIDTH);
                                        format!("width:{w}px")
                                    } />
                                }
                            }
                        />
                        <col />
                    </colgroup>
                    <thead>
                        <tr role="row">
                            {header}
                            <th
                                class="c-datatable__filler"
                                role="presentation"
                                aria-hidden="true"
                            ></th>
                        </tr>
                    </thead>
                    <tbody>
                        {(virtual_mode && !loading).then(|| {
                            view! {
                                <tr
                                    class="c-datatable__spacer"
                                    aria-hidden="true"
                                    style=move || {
                                        let (start, _) = clamp_win();
                                        format!("height:{}px", start as f64 * row_h)
                                    }
                                >
                                    <td colspan=move || (order.get().len() + 1).to_string()></td>
                                </tr>
                            }
                        })}
                        {loading.then(loading_view)}
                        <Show when=move || !loading && view_ids.get().is_empty()>
                            <tr class="c-datatable__row">
                                <td
                                    class="c-datatable__empty"
                                    colspan=move || (order.get().len() + 1).to_string()
                                >
                                    {empty_text.clone()}
                                </td>
                            </tr>
                        </Show>
                        <For
                            each=move || if loading { Vec::new() } else { window_rows.get() }
                            key=|row| row.id.clone()
                            children=move |row: DataTableRow| row_view(row)
                        />
                        {(virtual_mode && !loading).then(|| {
                            view! {
                                <tr
                                    class="c-datatable__spacer"
                                    aria-hidden="true"
                                    style=move || {
                                        let (_, end) = clamp_win();
                                        format!(
                                            "height:{}px",
                                            (row_count.saturating_sub(end)) as f64 * row_h,
                                        )
                                    }
                                >
                                    <td colspan=move || (order.get().len() + 1).to_string()></td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
            <div class="c-datatable__status" role="status" aria-live="polite">
                {move || {
                    if selection.is_selectable() {
                        format!(
                            "{} 行 · 已选 {}",
                            row_count,
                            sel.get().selected().len()
                        )
                    } else {
                        format!("{row_count} 行")
                    }
                }}
            </div>
        </div>
    }
}

/// 排序后的行（克隆一份，不碰调用方数据）。
fn sorted_rows(
    rows: &[DataTableRow],
    columns: &[DataColumn],
    sort: Option<(String, SortDir)>,
) -> Vec<DataTableRow> {
    let mut out = rows.to_vec();
    if let Some((key, dir)) = sort {
        if let Some(ci) = columns.iter().position(|c| c.key == key) {
            out.sort_by(|a, b| {
                let av = a.cells.get(ci).map(String::as_str).unwrap_or("");
                let bv = b.cells.get(ci).map(String::as_str).unwrap_or("");
                let ord = cmp_cell(av, bv);
                match dir {
                    SortDir::Asc => ord,
                    SortDir::Desc => ord.reverse(),
                }
            });
        }
    }
    out
}


/// 下一帧再执行。Leptos 的列重排在微任务里落定，焦点必须在它之后搬回去；
/// 浏览器在「移动已聚焦节点」时会先把焦点丢到 body，这一步不能省。
fn schedule(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}

fn focus_th(root: &web_sys::Element, key: &str) {
    let Ok(list) = root.query_selector_all(".c-datatable__th") else {
        return;
    };
    for i in 0..list.length() {
        let Some(node) = list.get(i) else { continue };
        let Ok(el) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if el.get_attribute("data-key").as_deref() == Some(key) {
            if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = html.focus();
            }
            return;
        }
    }
}

/// 表头单元格。抽成函数是因为里面的信号 handler 太多，写在组件体里会把
/// 主 view 淹掉，且每列都要克隆同一组句柄。
#[allow(clippy::too_many_arguments)]
fn header_cell(
    key: String,
    col: DataColumn,
    root: NodeRef<html::Div>,
    order: RwSignal<Vec<String>>,
    widths: RwSignal<HashMap<String, f64>>,
    sort: RwSignal<Option<(String, SortDir)>>,
    resizable: bool,
    drag_from: RwSignal<Option<usize>>,
    drag_start: RwSignal<(f64, f64)>,
    drag_active: RwSignal<bool>,
    drop_at: RwSignal<Option<usize>>,
    suppress_click: RwSignal<bool>,
    resizing_key: RwSignal<Option<String>>,
    resize_start: RwSignal<(f64, f64)>,
) -> AnyView {
    let sortable = col.sortable;
    let title = col.title.clone();
    let align = col.align;
    let min_width = if col.min_width > 0.0 {
        col.min_width
    } else {
        1.0
    };
    let init_width = normalize_width(&col);

    let key_aria = key.clone();
    let key_drop = key.clone();
    let key_kb = key.clone();
    let key_pd = key.clone();
    let key_sort = key.clone();
    let key_icon = key.clone();
    let key_resize = key.clone();

    let sort_button = sortable.then(|| {
        let key_sort = key_sort.clone();
        let key_icon = key_icon.clone();
        view! {
            <button
                class="c-datatable__sort"
                type="button"
                on:click=move |ev: web_sys::MouseEvent| {
                    ev.stop_propagation();
                    // 拖拽结束后浏览器补的 click：吃掉它，不改排序。
                    if suppress_click.get_untracked() {
                        suppress_click.set(false);
                        ev.prevent_default();
                        return;
                    }
                    let current = sort.get_untracked();
                    let next = match current {
                        Some((k, SortDir::Asc)) if k == key_sort => {
                            Some((k, SortDir::Desc))
                        }
                        Some((k, SortDir::Desc)) if k == key_sort => None,
                        _ => Some((key_sort.clone(), SortDir::Asc)),
                    };
                    sort.set(next);
                }
            >
                <span class="c-datatable__sort-label">{title.clone()}</span>
                {move || match sort.get() {
                    Some((k, SortDir::Asc)) if k == key_icon => {
                        view! { <Icon name=IconName::CHEVRON_UP size=11 /> }.into_any()
                    }
                    Some((k, SortDir::Desc)) if k == key_icon => {
                        view! { <Icon name=IconName::CHEVRON_DOWN size=11 /> }.into_any()
                    }
                    _ => view! { <Icon name=IconName::SORT size=11 /> }.into_any(),
                }}
            </button>
        }
        .into_any()
    });

    // 不可排序列没有按钮，标题仍要渲染；可排序列的标题在按钮内部（整块可点）。
    let static_label = (!sortable).then(|| {
        view! { <span class="c-datatable__sort-label">{title.clone()}</span> }.into_any()
    });

    let resizer = resizable.then(|| {
        let key_move = key_resize.clone();
        let key_up = key_resize.clone();
        let key_dbl = key_resize.clone();
        let key_kb = key_resize.clone();
        let key_pd = key_resize.clone();
        view! {
            <span
                class="c-datatable__resizer"
                role="separator"
                aria-orientation="vertical"
                aria-label="调整列宽"
                aria-valuemin=min_width.to_string()
                aria-valuenow=move || {
                    widths
                        .get()
                        .get(&key_move)
                        .copied()
                        .unwrap_or(init_width)
                        .round()
                        .to_string()
                }
                tabindex="0"
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    if ev.button() != 0 {
                        return;
                    }
                    ev.prevent_default();
                    // 不能让 th 的列拖拽也启动：一个手势只能有一个主人。
                    ev.stop_propagation();
                    if let Some(target) = ev.current_target() {
                        if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                            let _ = el.set_pointer_capture(ev.pointer_id());
                        }
                    }
                    let w = widths
                        .get_untracked()
                        .get(&key_pd)
                        .copied()
                        .unwrap_or(init_width);
                    resizing_key.set(Some(key_pd.clone()));
                    resize_start.set((ev.client_x() as f64, w));
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    if resizing_key.get_untracked().as_deref() != Some(key_up.as_str()) {
                        return;
                    }
                    let (sx, sw) = resize_start.get_untracked();
                    let next = (sw + (ev.client_x() as f64 - sx)).max(min_width).round();
                    widths.update(|m| {
                        m.insert(key_up.clone(), next);
                    });
                }
                on:pointerup=move |_| {
                    resizing_key.set(None);
                }
                on:pointercancel=move |_| {
                    resizing_key.set(None);
                }
                on:dblclick=move |ev: web_sys::MouseEvent| {
                    ev.prevent_default();
                    ev.stop_propagation();
                    widths.update(|m| {
                        m.insert(key_dbl.clone(), init_width);
                    });
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    // 命中区只有 7px，只用指针可达性不够：方向键也能调宽。
                    let cur = widths
                        .get_untracked()
                        .get(&key_kb)
                        .copied()
                        .unwrap_or(init_width);
                    let step = if ev.shift_key() { 32.0 } else { 16.0 };
                    let next = match ev.key().as_str() {
                        "ArrowLeft" => Some(cur - step),
                        "ArrowRight" => Some(cur + step),
                        "Home" => Some(min_width),
                        _ => None,
                    };
                    let Some(next) = next else { return };
                    ev.prevent_default();
                    ev.stop_propagation();
                    let next = next.max(min_width).round();
                    let key_now = key_kb.clone();
                    widths.update(|m| {
                        m.insert(key_now, next);
                    });
                }
            ></span>
        }
        .into_any()
    });

    view! {
        <th
            class="c-datatable__th"
            role="columnheader"
            tabindex="0"
            data-key=key.clone()
            data-align=align.as_str()
            aria-sort=move || {
                if !sortable {
                    return None;
                }
                Some(match sort.get() {
                    Some((k, SortDir::Asc)) if k == key_aria => "ascending",
                    Some((k, SortDir::Desc)) if k == key_aria => "descending",
                    _ => "none",
                })
            }
            data-drop=move || {
                if !drag_active.get() {
                    return None;
                }
                let list = order.get();
                let Some(index) = list.iter().position(|k| k == &key_drop) else {
                    return None;
                };
                match drop_at.get() {
                    Some(at) if at == index => Some("before"),
                    Some(at) if at == list.len() && index + 1 == list.len() => Some("after"),
                    _ => None,
                }
            }
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                if ev.button() != 0 {
                    return;
                }
                let Some(index) = order
                    .get_untracked()
                    .iter()
                    .position(|k| k == &key_pd)
                else {
                    return;
                };
                suppress_click.set(false);
                drag_from.set(Some(index));
                drag_start.set((ev.client_x() as f64, ev.client_y() as f64));
                drag_active.set(false);
                drop_at.set(None);
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                // 键盘替代：Alt+←/→ 交换相邻列。焦点在 th 内的排序按钮上时，
                // 事件冒泡到这里，同样有效。
                if !ev.alt_key() {
                    return;
                }
                let list = order.get_untracked();
                let Some(index) = list.iter().position(|k| k == &key_kb) else {
                    return;
                };
                let target = match ev.key().as_str() {
                    "ArrowLeft" => index.checked_sub(1),
                    "ArrowRight" if index + 1 < list.len() => Some(index + 1),
                    _ => return,
                };
                if let Some(to) = target {
                    ev.prevent_default();
                    order.update(|o| o.swap(index, to));
                    // 移动已聚焦的 th 会让浏览器把焦点丢到 body，下一帧搬回来。
                    let key_focus = key_kb.clone();
                    schedule(move || {
                        if let Some(root) = root.get_untracked() {
                            focus_th(&root, &key_focus);
                        }
                    });
                }
            }
        >
            {sort_button}
            {static_label}
            {resizer}
        </th>
    }
    .into_any()
}
