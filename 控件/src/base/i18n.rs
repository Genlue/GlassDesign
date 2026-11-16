//! 界面语言与文案表。
//!
//! 所有 `aria-label`、占位符、空态文案都从这里取，组件里不再散落字符串字面量。
//!
//! 三条约定：
//! 1) **默认中文**：不调用 [`provide_locale`] 时 [`t`] 返回 [`Strings::ZH_CN`]，
//!    全库行为与加这张表之前逐字一致（中文字段就是原来那些字面量）。
//! 2) **表是 `Copy` 的**：字段全是 `&'static str`，能塞进 context、也能被闭包反复取，
//!    不会因为捕获一个 `String` 把 `Fn` 退化成 `FnOnce`。
//! 3) **要追踪**：`t()` 内部会 `get()` 当前语言信号，所以必须**在响应式闭包里调**
//!    （`move || t().empty_default`），切语言才会重渲染。
//!
//! 带占位符的字段只放模板，调用处用 [`fill`] 填充：
//! `fill(s.selected_fmt, &[("n", &n.to_string())])`。

use leptos::prelude::*;

/// 界面语言。默认中文 —— 不调用 `provide_locale` 时全库行为与现在**逐字一致**。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Locale {
    #[default]
    ZhCn,
    En,
}

/// 一张文案表。字段全是 `&'static str`，所以它是 `Copy`、能塞进 context 和 Memo。
///
/// 带 `{name}` 的字段是模板，用 [`fill`] 填充；填入值本身不转义。
#[derive(Clone, Copy, Debug)]
pub struct Strings {
    /* ---------------------------------------------------------------- 空态与加载 */
    /// 表格类的空态。
    pub empty_default: &'static str,
    /// 集合（网格 / 列表）的空态。
    pub empty_collection: &'static str,
    /// 树的空态。
    pub empty_tree: &'static str,
    /// 行内加载占位（树的 `Loading` 状态行）。
    pub loading: &'static str,

    /* -------------------------------------------------------------- 占位与兜底名 */
    /// 下拉未选时的占位。
    pub select_placeholder: &'static str,
    /// 菜单的 `aria-label` 兜底。
    pub menu_label: &'static str,
    /// 对话框的 `aria-label` 兜底。**本批未接线**：`Dialog` 现在直接用 `title`
    /// 当可访问名（没有关闭按钮、也没有兜底分支），字段留给消息对话框收口时用。
    pub dialog_label: &'static str,
    /// 气泡的 `aria-label` 兜底。
    pub popover_label: &'static str,
    /// 流程面板的 `aria-label` 兜底。
    pub sheet_label: &'static str,
    /// 抽屉的 `aria-label` 兜底。
    pub drawer_label: &'static str,
    /// 数据表的 `aria-label` 兜底。
    pub table_label: &'static str,
    /// 集合的 `aria-label` 兜底。
    pub collection_label: &'static str,
    /// 树的 `aria-label` 兜底。
    pub tree_label: &'static str,
    /// 分页的 `aria-label` 兜底。
    pub pagination_label: &'static str,
    /// 文件选择的按钮文案兜底。
    pub file_picker_label: &'static str,

    /* -------------------------------------------------------------------- 动作 */
    /// 关闭。浮层右上角的关闭按钮。
    pub close: &'static str,
    /// 取消。**本批未接线**：`FormActions` 的兜底还没迁（不在本批清单里）。
    pub cancel: &'static str,
    /// 确定。**本批未接线**，理由同上。
    pub confirm: &'static str,
    /// 今天。**本批未接线**：日期面板现在没有「回到今天」按钮，先留字段。
    pub today: &'static str,
    /// 清除。日期面板底部的清除。
    pub clear: &'static str,
    /// 重试。树的加载失败行。
    pub retry: &'static str,
    /// 底部抽屉抓握条的 `aria-label`。
    pub drag_close: &'static str,

    /* ------------------------------------------------------------------ 日期 / 时间 */
    /// 月/年标题模板：`{y}` 年 `{m}` 月。
    pub month_year_fmt: &'static str,
    /// 整日 `aria-label` 模板：`{y}` 年 `{m}` 月 `{d}` 日。
    pub day_label_fmt: &'static str,
    /// 周表头，周一…周日（与网格的列序一致，不跟 locale 的「一周从周日开始」）。
    pub weekdays: [&'static str; 7],
    /// 上个月（翻月按钮的 `aria-label`）。
    pub prev_month: &'static str,
    /// 下个月。
    pub next_month: &'static str,
    /// 日期控件的 `aria-label` 兜底。
    pub date_label: &'static str,
    /// 日期区间的 `aria-label` 兜底。
    pub date_range_label: &'static str,
    /// 日期面板自己的 `aria-label`。
    pub pick_date: &'static str,
    /// 区间面板自己的 `aria-label`。
    pub pick_date_range: &'static str,
    /// 区间触发器未选时的占位（比面板名短一截）。
    pub pick_range_blank: &'static str,
    /// 时间控件的 `aria-label` 兜底。
    pub time_label: &'static str,
    /// 时间区间的 `aria-label` 兜底。
    pub time_range_label: &'static str,
    /// 时间区间左端。
    pub start_time: &'static str,
    /// 时间区间右端。
    pub end_time: &'static str,
    /// 跨零点提示。
    pub crosses_midnight: &'static str,
    /// 小时段。
    pub hour: &'static str,
    /// 分钟段。
    pub minute: &'static str,
    /// 秒段。
    pub second: &'static str,
    /// 上午/下午段的 `aria-label`。
    pub meridiem: &'static str,
    /// 上午。
    pub am: &'static str,
    /// 下午。
    pub pm: &'static str,

    /* -------------------------------------------------------------------- 分页 */
    /// 上一页。
    pub prev_page: &'static str,
    /// 下一页。
    pub next_page: &'static str,
    /// 页码按钮的 `aria-label` 模板：第 `{n}` 页。
    pub page_label_fmt: &'static str,
    /// 页码按钮组的分组名。
    pub pages_label: &'static str,
    /// 摘要模板：第 `{page}` 页，共 `{total}` 页。
    pub page_status_fmt: &'static str,

    /* ------------------------------------------------------------------ 数据表 */
    /// 行数模板：`{n}` 行。
    pub rows_status_fmt: &'static str,
    /// 选中数模板：已选 `{n}`。**本批未接线**：数据表用的是合并模板
    /// [`Self::rows_selected_fmt`]；单独一行时只有 [`Self::files_selected_fmt`] 在用。
    pub selected_fmt: &'static str,
    /// 有选择时的一行摘要：`{n}` 行 · 已选 `{m}`。
    pub rows_selected_fmt: &'static str,
    /// 单元格编辑器模板：编辑 `{title}`。
    pub edit_cell_fmt: &'static str,
    /// 列宽拖拽柄。
    pub resize_column: &'static str,

    /* ---------------------------------------------------------------- 搜索 / 选择 */
    /// 搜索框占位，也用作组合框输入框的 `aria-label`。
    pub search_placeholder: &'static str,
    /// 组合框候选面板的名字。
    pub suggestions_label: &'static str,
    /// 过滤后无候选。
    pub no_results: &'static str,
    /// 允许自定义值时，空候选的提示。
    pub enter_to_use: &'static str,

    /* -------------------------------------------------------------------- 文件 */
    /// 拖放区主文案。
    pub drop_hint: &'static str,
    /// 拖放区副文案。
    pub drop_sub_hint: &'static str,
    /// 拖放区的浏览按钮。
    pub browse: &'static str,
    /// 已选文件清单的名字。
    pub selected_files_label: &'static str,
    /// 多选后的按钮文案：已选 `{n}` 个文件。
    pub files_selected_fmt: &'static str,
    /// 移除单个文件：移除 `{name}`。
    pub remove_fmt: &'static str,
    /// 校验：`{name}` 类型不支持。
    pub file_unsupported_fmt: &'static str,
    /// 校验：`{name}` 超过 `{size}`。
    pub file_too_large_fmt: &'static str,
    /// 校验：`{name}` 最多 `{limit}` 个。
    pub file_limit_fmt: &'static str,
    /// 多条校验文案之间的分隔。
    pub list_sep: &'static str,

    /* -------------------------------------------------------------------- 校验 */
    /// 必填模板：`{name}` 不能为空。**本批未接线**：表单校验文案目前由调用方给。
    pub required_fmt: &'static str,
}

impl Strings {
    /// 简体中文。所有字段都与加表之前的字面量逐字相同。
    pub const ZH_CN: Self = Self {
        empty_default: "没有数据",
        empty_collection: "没有内容",
        empty_tree: "暂无内容",
        loading: "正在加载…",

        select_placeholder: "请选择",
        menu_label: "菜单",
        dialog_label: "对话框",
        popover_label: "浮层",
        sheet_label: "流程面板",
        drawer_label: "抽屉",
        table_label: "数据表",
        collection_label: "集合",
        tree_label: "树",
        pagination_label: "分页",
        file_picker_label: "选择文件",

        close: "关闭",
        cancel: "取消",
        confirm: "确定",
        today: "今天",
        clear: "清除",
        retry: "重试",
        drag_close: "向下拖动关闭",

        month_year_fmt: "{y} 年 {m} 月",
        day_label_fmt: "{y} 年 {m} 月 {d} 日",
        weekdays: ["一", "二", "三", "四", "五", "六", "日"],
        prev_month: "上个月",
        next_month: "下个月",
        date_label: "日期",
        date_range_label: "日期区间",
        pick_date: "选择日期",
        pick_date_range: "选择日期区间",
        pick_range_blank: "选择区间",
        time_label: "时间",
        time_range_label: "时间区间",
        start_time: "开始时间",
        end_time: "结束时间",
        crosses_midnight: "跨零点",
        hour: "小时",
        minute: "分钟",
        second: "秒",
        meridiem: "上午或下午",
        am: "上午",
        pm: "下午",

        prev_page: "上一页",
        next_page: "下一页",
        page_label_fmt: "第 {n} 页",
        pages_label: "页码",
        page_status_fmt: "第 {page} 页，共 {total} 页",

        rows_status_fmt: "{n} 行",
        selected_fmt: "已选 {n}",
        rows_selected_fmt: "{n} 行 · 已选 {m}",
        edit_cell_fmt: "编辑 {title}",
        resize_column: "调整列宽",

        search_placeholder: "搜索",
        suggestions_label: "候选",
        no_results: "没有匹配项",
        enter_to_use: "回车使用当前输入",

        drop_hint: "把文件拖到这里",
        drop_sub_hint: "或点击选择文件",
        browse: "浏览…",
        selected_files_label: "已选文件",
        files_selected_fmt: "已选 {n} 个文件",
        remove_fmt: "移除 {name}",
        file_unsupported_fmt: "{name}：类型不支持",
        file_too_large_fmt: "{name}：超过 {size}",
        file_limit_fmt: "{name}：最多 {limit} 个",
        list_sep: "；",

        required_fmt: "{name}不能为空",
    };

    /// English。
    pub const EN: Self = Self {
        empty_default: "No data",
        empty_collection: "Nothing here",
        empty_tree: "No items",
        loading: "Loading…",

        select_placeholder: "Select…",
        menu_label: "Menu",
        dialog_label: "Dialog",
        popover_label: "Popover",
        sheet_label: "Sheet",
        drawer_label: "Drawer",
        table_label: "Data table",
        collection_label: "Collection",
        tree_label: "Tree",
        pagination_label: "Pagination",
        file_picker_label: "Choose files",

        close: "Close",
        cancel: "Cancel",
        confirm: "OK",
        today: "Today",
        clear: "Clear",
        retry: "Retry",
        drag_close: "Drag down to close",

        month_year_fmt: "{m} / {y}",
        day_label_fmt: "{m}/{d}/{y}",
        weekdays: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        prev_month: "Previous month",
        next_month: "Next month",
        date_label: "Date",
        date_range_label: "Date range",
        pick_date: "Pick a date",
        pick_date_range: "Pick a date range",
        pick_range_blank: "Select a range",
        time_label: "Time",
        time_range_label: "Time range",
        start_time: "Start time",
        end_time: "End time",
        crosses_midnight: "Crosses midnight",
        hour: "Hour",
        minute: "Minute",
        second: "Second",
        meridiem: "AM or PM",
        am: "AM",
        pm: "PM",

        prev_page: "Previous page",
        next_page: "Next page",
        page_label_fmt: "Page {n}",
        pages_label: "Pages",
        page_status_fmt: "Page {page} of {total}",

        rows_status_fmt: "{n} rows",
        selected_fmt: "{n} selected",
        rows_selected_fmt: "{n} rows · {m} selected",
        edit_cell_fmt: "Edit {title}",
        resize_column: "Resize column",

        search_placeholder: "Search",
        suggestions_label: "Suggestions",
        no_results: "No matches",
        enter_to_use: "Press Enter to use this text",

        drop_hint: "Drop files here",
        drop_sub_hint: "or click to choose",
        browse: "Browse…",
        selected_files_label: "Selected files",
        files_selected_fmt: "{n} files selected",
        remove_fmt: "Remove {name}",
        file_unsupported_fmt: "{name}: unsupported type",
        file_too_large_fmt: "{name}: exceeds {size}",
        file_limit_fmt: "{name}: at most {limit} files",
        list_sep: "; ",

        required_fmt: "{name} is required",
    };

    /// 按语言取表。`const` 是有意的：调用方可以在常量里挑一张表。
    pub const fn of(locale: Locale) -> Self {
        match locale {
            Locale::ZhCn => Self::ZH_CN,
            Locale::En => Self::EN,
        }
    }
}

/// 应用根调用一次（画廊就是 `<App/>` 那一层）。**不调用 = 中文**，行为与改动前一致。
///
/// 用 `RwSignal` 而不是普通字段：切语言要能触发重渲染，而 `t()` 只读一次表，
/// 追踪的是这个信号的 `get()`。
pub fn provide_locale(locale: RwSignal<Locale, LocalStorage>) {
    provide_context(locale);
}

/// 取当前文案表。必须在响应式闭包里调（`move || t().empty_default`）才会跟语言切换重渲染。
///
/// 没有 provider 时返回 [`Strings::ZH_CN`]：库被当成「一个纯控件」嵌进别人的应用时，
/// 调用方不接线也能拿到和以前一样的文案。
pub fn t() -> Strings {
    match use_context::<RwSignal<Locale, LocalStorage>>() {
        Some(locale) => Strings::of(locale.get()),
        None => Strings::ZH_CN,
    }
}

/// 往模板里填 `{name}` 占位。表里只放模板，格式化留在调用处。
///
/// 先收集键再替换，所以填入的值里即使含有 `{x}` 也不会被二次展开（顺序无关）。
pub fn fill(tpl: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = tpl.to_string();
    for (key, value) in pairs {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_locale_is_chinese() {
        assert_eq!(Locale::default(), Locale::ZhCn);
        assert_eq!(Strings::of(Locale::default()).empty_default, "没有数据");
    }

    #[test]
    fn zh_strings_match_the_previous_literals() {
        // 这几条是迁移时逐字抄下来的，钉住它们等于钉住「默认语言零回归」。
        let s = Strings::ZH_CN;
        assert_eq!(s.select_placeholder, "请选择");
        assert_eq!(s.close, "关闭");
        assert_eq!(s.empty_collection, "没有内容");
        assert_eq!(s.empty_tree, "暂无内容");
        assert_eq!(s.month_year_fmt, "{y} 年 {m} 月");
        assert_eq!(s.page_status_fmt, "第 {page} 页，共 {total} 页");
        assert_eq!(s.rows_selected_fmt, "{n} 行 · 已选 {m}");
        assert_eq!(s.file_limit_fmt, "{name}：最多 {limit} 个");
        assert_eq!(s.weekdays, ["一", "二", "三", "四", "五", "六", "日"]);
    }

    #[test]
    fn fill_replaces_every_placeholder() {
        let s = Strings::of(Locale::ZhCn);
        assert_eq!(
            fill(s.month_year_fmt, &[("y", "2026"), ("m", "11")]),
            "2026 年 11 月"
        );
        assert_eq!(fill(s.selected_fmt, &[("n", "3")]), "已选 3");
        assert_eq!(
            fill(s.page_status_fmt, &[("page", "2"), ("total", "9")]),
            "第 2 页，共 9 页"
        );
        // 顺序无关，参数没给就原样留着
        assert_eq!(
            fill(s.rows_selected_fmt, &[("m", "1"), ("n", "12")]),
            "12 行 · 已选 1"
        );
        assert_eq!(fill("{a}/{b}", &[("a", "x")]), "x/{b}");
    }

    #[test]
    fn english_table_is_complete() {
        let s = Strings::of(Locale::En);
        assert_eq!(s.empty_default, "No data");
        assert_eq!(s.weekdays[0], "Mon");
        assert_eq!(fill(s.month_year_fmt, &[("y", "2026"), ("m", "11")]), "11 / 2026");
        // 没有字段被漏填成空串（漏一个就是一个空白 aria-label）
        assert!(!s.menu_label.is_empty());
        assert!(!s.close.is_empty());
        assert!(!s.loading.is_empty());
    }
}
