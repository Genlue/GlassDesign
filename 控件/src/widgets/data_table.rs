//! 数据表：列宽 / 列序 / 排序 / 选择 / 虚拟行 / 行内编辑。
//!
//! 与 `table.rs` 的分界沿用旧库：`Table` 是「读」（一次渲染，不碰状态），
//! `DataTable` 是「改」。两者合在一起会让只读表格背上一堆用不到的分支。
//!
//! 几条刻意的取舍：
//! 1) 列宽、列序只存在组件内部 —— 调用方传进来的 `columns` / `rows` 是数据，
//!    不是控件状态。拖动一次列宽不该改写调用方的常量。
//! 2) 选中集与排序是**双轨**：传了受控 prop（`selected` / `on_sort_change`）就只认它，
//!    点击只上报；没传就走内部状态。两套都是「状态在调用方」，但后者保证老代码一行不改。
//! 3) 选中集用稳定 `id`，排序 / 虚拟滚动都不会丢选中。
//! 4) 虚拟滚动用「上下两个撑高行」，而不是绝对定位：table 的布局引擎本来
//!    就按行高排队，绝对定位还得手算每行 top，并和 sticky 表头对账。
//! 5) 行内编辑是独立状态机（草稿 / 校验 / 输入法），但**不许改行高**：虚拟化的
//!    窗口数学只认固定行高（红线 5），所以输入框是绝对定位压在单元格里的。

use std::cmp::Ordering;
use std::collections::HashMap;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::i18n::{fill, t};
use crate::base::icon::{Icon, IconName};
use crate::base::virtual_window::{self, Window};
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
    /// 这一列能否行内编辑。默认 `false`：只读列不该因为旁边有人可编辑就变成输入框。
    pub editable: bool,
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
            editable: false,
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

    pub fn editable(mut self) -> Self {
        self.editable = true;
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

/// 排序规格。方向复用 `widgets::table::SortDir` —— 两张表在同一个页面里必须同向。
#[derive(Clone, PartialEq, Debug)]
pub struct SortSpec {
    pub key: String,
    pub dir: SortDir,
}

impl SortSpec {
    pub fn new(key: impl Into<String>, dir: SortDir) -> Self {
        Self {
            key: key.into(),
            dir,
        }
    }
}

/// 一次单元格提交。`value` 是**原始字符串**：类型转换、trim、格式化都由调用方决定，
/// 组件只负责「用户按下确认时这一格里是什么」。
#[derive(Clone, PartialEq, Debug)]
pub struct CellEdit {
    pub row_id: String,
    pub key: String,
    pub value: String,
}

/// 正在编辑的单元格。同一时刻只允许一个，所以不需要集合。
#[derive(Clone, PartialEq, Debug)]
struct CellPos {
    row: String,
    key: String,
}

fn is_cell(pos: Option<&CellPos>, row: &str, key: &str) -> bool {
    pos.is_some_and(|p| p.row == row && p.key == key)
}

/// 默认宽度。与旧库一致，避免调用方不写宽度时列被内容反推。
const DEFAULT_COL_WIDTH: f64 = 160.0;

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

/// 表头点击后的下一档排序。
///
/// 本地模式沿用旧行为：无序 → 升 → 降 → 无序（第三下清除）。
/// 服务端模式没有「清除」这一档：`on_sort_change: Callback<SortSpec>` 表达不了 `None`，
/// 所以降序再点回到升序（两态循环）；要清除由调用方把 `sort` 置空。
fn cycle_sort(current: Option<&SortSpec>, key: &str, server: bool) -> Option<SortSpec> {
    match current {
        Some(s) if s.key == key => match s.dir {
            SortDir::Asc => Some(SortSpec::new(key, SortDir::Desc)),
            SortDir::Desc if server => Some(SortSpec::new(key, SortDir::Asc)),
            SortDir::Desc => None,
        },
        _ => Some(SortSpec::new(key, SortDir::Asc)),
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
    /// 受控选中集：**传了就是受控**（渲染只认它，点击只上报），
    /// 不传就沿用 `default_selected` 作初值的内部状态（老行为）。
    #[prop(into, optional)] selected: MaybeProp<Vec<String>>,
    #[prop(into, optional)] default_selected: Vec<String>,
    #[prop(optional)] on_selection_change: Option<Callback<Vec<String>>>,
    /// 受控排序：只决定表头箭头与 `aria-sort`。
    #[prop(into, optional)] sort: MaybeProp<SortSpec>,
    /// 传了它就不在本地排序：`rows` 按调用方给的顺序渲染，组件只画箭头、只上报。
    #[prop(optional)] on_sort_change: Option<Callback<SortSpec>>,
    /// 单元格提交（Enter / 失焦 / 编辑中被虚拟窗口卸载）。
    #[prop(optional)] on_cell_commit: Option<Callback<CellEdit>>,
    /// 校验钩子：返回 `Some(文案)` 时**保持编辑态**、不提交。
    #[prop(optional)] validate: Option<Callback<CellEdit, Option<String>>>,
    /// 开启虚拟行时必须给固定行高；CSS 的 `--dt-row-h` 与窗口计算共用它。
    #[prop(optional)] virtual_row_height: Option<f64>,
    #[prop(optional)] loading: bool,
    #[prop(default = 6)] loading_rows: usize,
    #[prop(into, optional)] empty_text: String,
    #[prop(into, optional)] label: String,
    #[prop(default = true)] resizable: bool,
) -> impl IntoView {
    // 兜底值查文案表，且留在属性 / 文本闭包里：切语言时这两处要跟着变。
    let label_store = StoredValue::new_local(label);
    let empty_store = StoredValue::new_local(empty_text);
    let label = move || {
        let own = label_store.get_value();
        if own.is_empty() {
            t().table_label.to_string()
        } else {
            own
        }
    };
    let empty_text = move || {
        let own = empty_store.get_value();
        if own.is_empty() {
            t().empty_default.to_string()
        } else {
            own
        }
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
    let editable_cols: HashMap<String, bool> = columns
        .iter()
        .map(|c| (c.key.clone(), c.editable))
        .collect();

    let order = RwSignal::new(columns.iter().map(|c| c.key.clone()).collect::<Vec<_>>());
    let widths = RwSignal::new(
        columns
            .iter()
            .map(|c| (c.key.clone(), normalize_width(c)))
            .collect::<HashMap<String, f64>>(),
    );

    /* -------------------------------------------------- 选择：受控 / 非受控 */

    // 「受控与否」只在挂载时判定一次：`MaybeProp` 分不清「没传」和「传了 None」，
    // 而中途换轨只会让选中集跳一下，没有好处。
    let controlled_sel = selected.get_untracked().is_some();
    let sel = RwSignal::new(SelectionState::with_selected(default_selected));
    let sel_ids = Memo::new(move |_| {
        if controlled_sel {
            selected.get().unwrap_or_default()
        } else {
            sel.get().selected().to_vec()
        }
    });

    /* -------------------------------------------------- 排序：受控 / 本地 */

    let server_sort = on_sort_change.is_some();
    let local_sort = RwSignal::new(sort.get_untracked());
    // 表头箭头只认这一个：受控模式下就是 prop，本地模式下是内部状态。
    let shown_sort = Memo::new(move |_| {
        if server_sort {
            sort.get()
        } else {
            local_sort.get()
        }
    });
    // 真正喂给 `sorted_rows` 的值：服务端模式下恒为 None（行序由调用方决定）。
    let sort_for_rows = Memo::new(move |_| if server_sort { None } else { shown_sort.get() });

    let toggle_sort = Callback::new(move |key: String| {
        let next = cycle_sort(shown_sort.get_untracked().as_ref(), &key, server_sort);
        if server_sort {
            // 两态循环下 `next` 永远是 Some，不存在「上报一次清除」这种动作。
            if let (Some(spec), Some(cb)) = (next, on_sort_change) {
                cb.run(spec);
            }
        } else {
            local_sort.set(next);
        }
    });

    let sort_sig: Signal<Option<SortSpec>> = shown_sort.into();

    let row_count = rows.len();

    let ids_rows = rows.clone();
    let ids_cols = columns.clone();
    let view_ids = Memo::new(move |_| {
        // 排序后的稳定 ID 序列：Shift 连选、↑/↓ 移动焦点都必须按「当前看到」的顺序走。
        sorted_rows(&ids_rows, &ids_cols, sort_for_rows.get())
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
    });

    /* ---------------------------------------------------------- 虚拟窗口 */

    // 窗口算法在 `base/virtual_window`：树与集合复用同一份，边界只在那边维护。
    let win = RwSignal::new(if virtual_mode {
        Window::new(0, 18.min(row_count))
    } else {
        Window::full(row_count)
    });

    let clamp_win = move || virtual_window::clamp(win.get(), row_count);

    let win_rows = rows.clone();
    let win_cols = columns.clone();
    let window_rows = Memo::new(move |_| {
        let all = sorted_rows(&win_rows, &win_cols, sort_for_rows.get());
        if !virtual_mode {
            return all;
        }
        all[clamp_win().slice()].to_vec()
    });

    let on_scroll = move |_| {
        if !virtual_mode {
            return;
        }
        let Some(el) = scroll_ref.get_untracked() else {
            return;
        };
        let next = virtual_window::window_of(&el, row_h, row_count, virtual_window::OVERSCAN);
        if win.get_untracked() != next {
            win.set(next);
        }
    };

    /* ------------------------------------------------------ 行焦点（roving） */

    // 焦点与选中分离：↑/↓ 只搬焦点，Space / 点击才改选中（与 `selection.rs` 约定一致）。
    let focus_row = RwSignal::new(None::<String>);
    let effective_focus = Memo::new(move |_| {
        let ids = view_ids.get();
        match focus_row.get() {
            Some(k) if ids.iter().any(|x| x == &k) => Some(k),
            _ => ids.first().cloned(),
        }
    });
    // 全表只有一个 `tabindex="0"`，且它必须落在**已渲染**的行里：虚拟模式下焦点行
    // 滚出窗口后如果没人接这一位，整张表就从 Tab 序列里消失了。
    let tab_row = Memo::new(move |_| {
        let visible = window_rows.get();
        match effective_focus.get() {
            Some(k) if visible.iter().any(|r| r.id == k) => Some(k),
            _ => visible.first().map(|r| r.id.clone()),
        }
    });

    // 让第 `index` 行成为焦点行：虚拟模式下先滚进视口（窗口才会包含它），
    // 下一帧再把 DOM 焦点搬过去。
    let move_focus = move |delta: i64| {
        let ids = view_ids.get_untracked();
        if ids.is_empty() {
            return;
        }
        let next = match effective_focus
            .get_untracked()
            .and_then(|k| ids.iter().position(|x| *x == k))
        {
            Some(i) => (i as i64 + delta).clamp(0, ids.len() as i64 - 1) as usize,
            None if delta >= 0 => 0,
            None => ids.len() - 1,
        };
        let id = ids[next].clone();
        if focus_row.get_untracked().as_deref() != Some(id.as_str()) {
            focus_row.set(Some(id.clone()));
        }
        if virtual_mode {
            if let Some(el) = scroll_ref.get_untracked() {
                ensure_visible(&el, next, row_h);
                let w = virtual_window::window_of(&el, row_h, row_count, virtual_window::OVERSCAN);
                if win.get_untracked() != w {
                    win.set(w);
                }
            }
        }
        schedule(move || {
            if let Some(root) = root_ref.get_untracked() {
                focus_row_el(&root, &id);
            }
        });
    };

    /* ------------------------------------------------------------ 行内编辑 */

    let editing = RwSignal::new(None::<CellPos>);
    let draft = RwSignal::new(String::new());
    let edit_err = RwSignal::new(None::<String>);

    // `refocus`：只有键盘 Enter 才把焦点搬回行；失焦提交时用户已经点到别处了，
    // 再抢一次焦点就是「焦点打架」。
    let commit_edit = Callback::new(move |refocus: bool| {
        let Some(pos) = editing.get_untracked() else {
            return;
        };
        let edit = CellEdit {
            row_id: pos.row.clone(),
            key: pos.key.clone(),
            value: draft.get_untracked(),
        };
        if let Some(check) = validate {
            if let Some(msg) = check.run(edit.clone()) {
                // 校验不过：保持编辑态，文案挂到单元格外面（不出这一格）。
                edit_err.set(Some(msg));
                return;
            }
        }
        editing.set(None);
        edit_err.set(None);
        if let Some(cb) = on_cell_commit {
            cb.run(edit);
        }
        if refocus {
            let row = pos.row;
            focus_row.set(Some(row.clone()));
            schedule(move || {
                if let Some(root) = root_ref.get_untracked() {
                    focus_row_el(&root, &row);
                }
            });
        }
    });

    let begin_edit = Callback::new(move |(row, key, initial): (String, String, String)| {
        // 换格之前先把上一格按 Enter 语义提交：同一时刻只允许一个编辑态，
        // 但「换格」不该等于「悄悄丢弃刚打的字」。校验没过就留在原格。
        if editing.get_untracked().is_some() {
            commit_edit.run(false);
            if editing.get_untracked().is_some() {
                return;
            }
        }
        // 草稿先落地：编辑态一变，单元格立刻重渲染并把它读走。
        draft.set(initial);
        edit_err.set(None);
        editing.set(Some(CellPos { row, key }));
        schedule(move || {
            if let Some(root) = root_ref.get_untracked() {
                focus_editor(&root);
            }
        });
    });

    let cancel_edit = Callback::new(move |_: ()| {
        let Some(pos) = editing.get_untracked() else {
            return;
        };
        editing.set(None);
        edit_err.set(None);
        let row = pos.row;
        focus_row.set(Some(row.clone()));
        schedule(move || {
            if let Some(root) = root_ref.get_untracked() {
                focus_row_el(&root, &row);
            }
        });
    });

    // 编辑中的单元格被窗口甩掉：先按 Enter 语义提交，再让它随窗口一起卸载。
    // 这里不走 `commit_edit`：那一支在「校验失败」时会保留编辑态，而这一格马上
    // 就不在 DOM 里了 —— 看不见的编辑态比丢草稿更糟。所以本格先清态、再提交，
    // 且回调放在最后一句（回调可能让调用方重挂载整张表）。
    Effect::new(move |_| {
        if !virtual_mode {
            return;
        }
        let Some(pos) = editing.get() else {
            return;
        };
        if window_rows.get().iter().any(|r| r.id == pos.row) {
            return;
        }
        let edit = CellEdit {
            row_id: pos.row.clone(),
            key: pos.key.clone(),
            value: draft.get_untracked(),
        };
        let ok = match validate {
            Some(check) => check.run(edit.clone()).is_none(),
            None => true,
        };
        editing.set(None);
        edit_err.set(None);
        if ok {
            if let Some(cb) = on_cell_commit {
                cb.run(edit);
            }
        }
    });

    /* ------------------------------------------------------------ 选择动作 */

    let pick = Callback::new(move |(id, toggle, range): (String, bool, bool)| {
        if !selection.is_selectable() {
            return;
        }
        let order = view_ids.get_untracked();
        // 非受控沿用内部状态（Shift 锚点要跨点击保持）；受控每次都从外部值重建，
        // 因为「调用方没接受这次变更」时内部副本会悄悄跑偏。
        let mut next = if controlled_sel {
            SelectionState::with_selected(selected.get_untracked().unwrap_or_default())
        } else {
            sel.get_untracked()
        };
        if !next.click(selection, &id, &order, toggle, range) {
            return;
        }
        let out = next.ordered(&order);
        if !controlled_sel {
            sel.set(next);
        }
        if let Some(cb) = on_selection_change {
            cb.run(out);
        }
    });

    let select_all = Callback::new(move |_: ()| {
        if !selection.is_multiple() {
            return;
        }
        let order = view_ids.get_untracked();
        let mut next = if controlled_sel {
            SelectionState::with_selected(selected.get_untracked().unwrap_or_default())
        } else {
            sel.get_untracked()
        };
        if !next.select_all(&order) {
            return;
        }
        let out = next.ordered(&order);
        if !controlled_sel {
            sel.set(next);
        }
        if let Some(cb) = on_selection_change {
            cb.run(out);
        }
    });

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
                            sort_sig,
                            toggle_sort,
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

    /* ------------------------------------------------------------ Enter 编辑 */

    // 焦点行上按 Enter / F2 编辑「第一个可编辑列」。列表按当前列序算，拖过列也对。
    let first_editable = Memo::new(move |_| {
        order
            .get()
            .into_iter()
            .find(|k| editable_cols.get(k).copied().unwrap_or(false))
    });

    /* --------------------------------------------------------------- 行 */

    let row_view = {
        let col_index = col_index.clone();
        let columns = columns.clone();
        let mode = selection;
        let sel_ids = sel_ids;
        let order_sig = order;
        move |row: DataTableRow| -> AnyView {
            let id_click = row.id.clone();
            let id_sel = row.id.clone();
            let id_key = row.id.clone();
            let id_focus = row.id.clone();
            let id_tab = row.id.clone();
            let id_kb = row.id.clone();
            let cells_row = row.clone();
            // keydown 里的 Enter/F2 要读「这一行第一个可编辑列的当前值」，
            // 而单元格 For 也要读同一份数据：各自一份克隆，别让 For 的 move 抢走。
            let cells_kb = row.clone();
            let cols = columns.clone();
            let idx_map = col_index.clone();
            let idx_kb = col_index.clone();
            view! {
                <tr
                    class="c-datatable__row"
                    role="row"
                    data-key=id_key
                    tabindex=move || {
                        if tab_row.get().as_deref() == Some(id_tab.as_str()) { "0" } else { "-1" }
                    }
                    aria-selected=move || {
                        mode.is_selectable()
                            .then(|| sel_ids.get().iter().any(|k| k == &id_sel).to_string())
                    }
                    on:focus=move |_| focus_row.set(Some(id_focus.clone()))
                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                        if ev.button() != 0 || !mode.is_selectable() {
                            return;
                        }
                        // 行里可能放按钮 / 链接 / 编辑框，那些区域不归选择管。
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
                        pick.run((
                            id_click.clone(),
                            ev.ctrl_key() || ev.meta_key(),
                            ev.shift_key(),
                        ));
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        // 输入法组合态里的按键属于候选词，不属于表格。
                        if ev.is_composing() || ev.key_code() == 229 {
                            return;
                        }
                        if let Some(target) = ev.target() {
                            if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                                if el
                                    .closest("input, textarea, select, button, a")
                                    .ok()
                                    .flatten()
                                    .is_some()
                                {
                                    return;
                                }
                            }
                        }
                        match ev.key().as_str() {
                            "ArrowDown" => {
                                ev.prevent_default();
                                move_focus(1);
                            }
                            "ArrowUp" => {
                                ev.prevent_default();
                                move_focus(-1);
                            }
                            " " | "Spacebar" => {
                                if !mode.is_selectable() {
                                    return;
                                }
                                // 不拦就是翻页。
                                ev.prevent_default();
                                pick.run((id_kb.clone(), true, false));
                            }
                            "a" | "A" if ev.ctrl_key() || ev.meta_key() => {
                                if !mode.is_multiple() {
                                    return;
                                }
                                ev.prevent_default();
                                select_all.run(());
                            }
                            "Enter" | "F2" => {
                                let Some(key) = first_editable.get() else {
                                    return;
                                };
                                ev.prevent_default();
                                let ci = idx_kb.get(&key).copied().unwrap_or(usize::MAX);
                                let text = cells_kb.cells.get(ci).cloned().unwrap_or_default();
                                begin_edit.run((id_kb.clone(), key, text));
                            }
                            _ => {}
                        }
                    }
                >
                    <For
                        each=move || order_sig.get()
                        key=|k| k.clone()
                        children=move |key: String| {
                            let ci = idx_map.get(&key).copied().unwrap_or(usize::MAX);
                            let col = cols.get(ci);
                            body_cell(
                                cells_row.id.clone(),
                                key,
                                cells_row.cells.get(ci).cloned().unwrap_or_default(),
                                col.map(|c| c.align).unwrap_or(ColAlign::Left),
                                col.map(|c| c.editable).unwrap_or(false),
                                col.map(|c| c.title.clone()).unwrap_or_default(),
                                editing,
                                draft,
                                edit_err,
                                begin_edit,
                                commit_edit,
                                cancel_edit,
                            )
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
                                        let start = clamp_win().start();
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
                                    {empty_text}
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
                                        let end = clamp_win().end();
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
                    let n = row_count.to_string();
                    if selection.is_selectable() {
                        let m = sel_ids.get().len().to_string();
                        fill(t().rows_selected_fmt, &[("n", &n), ("m", &m)])
                    } else {
                        fill(t().rows_status_fmt, &[("n", &n)])
                    }
                }}
            </div>
        </div>
    }
}

/// 排序后的行（克隆一份，不碰调用方数据）。服务端排序时调用方传 `None`，原样返回。
fn sorted_rows(
    rows: &[DataTableRow],
    columns: &[DataColumn],
    sort: Option<SortSpec>,
) -> Vec<DataTableRow> {
    let mut out = rows.to_vec();
    if let Some(spec) = sort {
        if let Some(ci) = columns.iter().position(|c| c.key == spec.key) {
            out.sort_by(|a, b| {
                let av = a.cells.get(ci).map(String::as_str).unwrap_or("");
                let bv = b.cells.get(ci).map(String::as_str).unwrap_or("");
                let ord = cmp_cell(av, bv);
                match spec.dir {
                    SortDir::Asc => ord,
                    SortDir::Desc => ord.reverse(),
                }
            });
        }
    }
    out
}

/// 一行里的一格。抽成函数是因为「可编辑 / 只读」两条分支的 handler 数量差一倍，
/// 写在主 view 里会把结构淹掉。
#[allow(clippy::too_many_arguments)]
fn body_cell(
    row_id: String,
    key: String,
    text: String,
    align: ColAlign,
    editable: bool,
    title: String,
    editing: RwSignal<Option<CellPos>>,
    draft: RwSignal<String>,
    edit_err: RwSignal<Option<String>>,
    begin: Callback<(String, String, String)>,
    commit: Callback<bool>,
    cancel: Callback<()>,
) -> AnyView {
    if !editable {
        // 只读格也带 `aria-readonly`：读屏要能区分「这一格不能改」与「能改但没在改」。
        return view! {
            <td
                class="c-datatable__cell"
                role="gridcell"
                data-align=align.as_str()
                aria-readonly="true"
            >
                {text}
            </td>
        }
        .into_any();
    }

    let r_m = row_id.clone();
    let k_m = key.clone();
    let on_this = Memo::new(move |_| is_cell(editing.get().as_ref(), &r_m, &k_m));

    let id_dbl = row_id.clone();
    let key_dbl = key.clone();
    let text_dbl = text.clone();
    let r_blur = row_id;
    let k_blur = key;

    view! {
        <td
            class="c-datatable__cell"
            role="gridcell"
            data-align=align.as_str()
            aria-readonly="false"
            data-editing=move || on_this.get().then_some("1")
            aria-invalid=move || (on_this.get() && edit_err.get().is_some()).then_some("true")
            on:dblclick=move |ev: web_sys::MouseEvent| {
                ev.prevent_default();
                begin.run((id_dbl.clone(), key_dbl.clone(), text_dbl.clone()));
            }
        >
            {move || {
                if on_this.get() {
                    // 这一支每次重渲染都要全新的克隆：闭包是 `Fn`，不能把外层的
                    // String 一次性搬进来（否则整支退化成 `FnOnce`）。
                    let r_blur = r_blur.clone();
                    let k_blur = k_blur.clone();
                    // `value` 只在这一支重渲染时取一次草稿：绑成响应式会在每次
                    // 输入时重写 value，把光标和输入法的组合态一起打断。
                    view! {
                        <input
                            class="c-datatable__editor"
                            type="text"
                            value=draft.get_untracked()
                            aria-label=fill(t().edit_cell_fmt, &[("title", &title)])
                            on:input=move |ev| {
                                draft.set(event_target_value(&ev));
                                // 用户已经在改了：上一次的报错不再成立，先收起来。
                                // （重新提交时校验会再判一次。）
                                if edit_err.get_untracked().is_some() {
                                    edit_err.set(None);
                                }
                            }
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                // 输入法组合态里的 Enter 是「确认候选」，不能拿来提交
                                // （与 menu.rs 同一处理，原因见 docs/03 排障表）。
                                if ev.is_composing() || ev.key_code() == 229 {
                                    ev.stop_propagation();
                                    return;
                                }
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        commit.run(true);
                                    }
                                    "Escape" => {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        cancel.run(());
                                    }
                                    // 光标要在输入框里走，别冒泡成「移动行焦点」
                                    "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
                                    | " " | "Spacebar" => ev.stop_propagation(),
                                    _ => {}
                                }
                            }
                            on:blur=move |_| {
                                // 只有「还是这一格在编辑」才提交：取消 / 卸载时
                                // editing 已经清空，不能补一发迟到的提交。
                                if is_cell(editing.get_untracked().as_ref(), &r_blur, &k_blur) {
                                    commit.run(false);
                                }
                            }
                        />
                        {move || {
                            edit_err
                                .get()
                                .map(|msg| {
                                    view! {
                                        <span class="c-datatable__cell-error" role="alert">
                                            {msg}
                                        </span>
                                    }
                                })
                        }}
                    }
                        .into_any()
                } else {
                    text.clone().into_any()
                }
            }}
        </td>
    }
    .into_any()
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

/// 把 DOM 焦点搬到行上。没有用 `focus({preventScroll:true})`：web-sys 没开
/// `FocusOptions` 特性；效果等价的做法是「先自己最小滚动，再 focus」
/// （原因见 docs/03 排障表：focus 默认居中会把连按 ↓ 的人拽回去）。
fn focus_row_el(root: &web_sys::Element, id: &str) {
    let Ok(list) = root.query_selector_all(".c-datatable__row") else {
        return;
    };
    for i in 0..list.length() {
        let Some(node) = list.get(i) else { continue };
        let Ok(el) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if el.get_attribute("data-key").as_deref() == Some(id) {
            if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = html.focus();
            }
            return;
        }
    }
}

/// 让第 `index` 行落进视口，只做最小滚动。表头是 sticky 的，它压住的那一段
/// 不算可见区（量不到就当 0，宁可多滚一像素）。
fn ensure_visible(scroll: &web_sys::Element, index: usize, row_h: f64) {
    let head_h = scroll
        .query_selector(".c-datatable__th")
        .ok()
        .flatten()
        .map(|th| th.get_bounding_client_rect().height())
        .unwrap_or(0.0);
    let view_h = scroll.client_height() as f64;
    let top = scroll.scroll_top() as f64;
    let y = head_h + index as f64 * row_h;
    let next = if y < top + head_h {
        y - head_h
    } else if y + row_h > top + view_h {
        y + row_h - view_h
    } else {
        top
    };
    if next != top {
        scroll.set_scroll_top(next.max(0.0).round() as i32);
    }
}

/// 编辑态刚出现时把焦点送进输入框并全选：整值替换比「点进光标位置」更常用，
/// 和 F2 / Enter 的语义也一致。
fn focus_editor(root: &web_sys::Element) {
    let Ok(Some(node)) = root.query_selector(".c-datatable__cell[data-editing=\"1\"] .c-datatable__editor")
    else {
        return;
    };
    let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
        return;
    };
    let _ = input.focus();
    input.select();
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
    sort: Signal<Option<SortSpec>>,
    on_toggle: Callback<String>,
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
                    on_toggle.run(key_sort.clone());
                }
            >
                <span class="c-datatable__sort-label">{title.clone()}</span>
                {move || match sort.get() {
                    Some(s) if s.key == key_icon && s.dir == SortDir::Asc => {
                        view! { <Icon name=IconName::CHEVRON_UP size=11 /> }.into_any()
                    }
                    Some(s) if s.key == key_icon && s.dir == SortDir::Desc => {
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
                aria-label=move || t().resize_column.to_string()
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
                    Some(s) if s.key == key_aria && s.dir == SortDir::Asc => "ascending",
                    Some(s) if s.key == key_aria && s.dir == SortDir::Desc => "descending",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_cells_sort_numerically() {
        assert_eq!(cmp_cell("9", "10"), Ordering::Less);
        assert_eq!(cmp_cell("10", "9"), Ordering::Greater);
    }

    #[test]
    fn empty_cells_sink() {
        // 空值排在有值之后（升序里沉底）：true.cmp(false) == Greater
        assert_eq!(cmp_cell("", "a"), Ordering::Greater);
        assert_eq!(cmp_cell("a", ""), Ordering::Less);
        assert_eq!(cmp_cell("", ""), Ordering::Equal);
    }

    #[test]
    fn local_cycle_is_three_state() {
        let asc = cycle_sort(None, "name", false).unwrap();
        assert_eq!(asc.dir, SortDir::Asc);
        let desc = cycle_sort(Some(&asc), "name", false).unwrap();
        assert_eq!(desc.dir, SortDir::Desc);
        // 本地模式第三下清除
        assert!(cycle_sort(Some(&desc), "name", false).is_none());
        // 换一列从头开始
        assert_eq!(
            cycle_sort(Some(&desc), "size", false).unwrap().dir,
            SortDir::Asc
        );
    }

    #[test]
    fn server_cycle_never_clears() {
        let desc = SortSpec::new("name", SortDir::Desc);
        let next = cycle_sort(Some(&desc), "name", true).unwrap();
        assert_eq!(next.dir, SortDir::Asc);
    }

    #[test]
    fn sorted_rows_ignores_unknown_key() {
        let rows = vec![
            DataTableRow::new("b", vec!["2"]),
            DataTableRow::new("a", vec!["1"]),
        ];
        let cols = vec![DataColumn::new("n", "N")];
        let out = sorted_rows(&rows, &cols, Some(SortSpec::new("missing", SortDir::Asc)));
        assert_eq!(
            out.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["b", "a"]
        );
        let out = sorted_rows(&rows, &cols, Some(SortSpec::new("n", SortDir::Asc)));
        assert_eq!(
            out.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
}
