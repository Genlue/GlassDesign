use glass_design::prelude::*;
use leptos::prelude::*;

#[component]
pub fn DataPage() -> impl IntoView {
    let selected = RwSignal::new(Some("2".to_string()));
    let page = RwSignal::new(2u32);
    let sort = RwSignal::new(Some(TableSort {
        key: "name".to_string(),
        dir: SortDir::Asc,
    }));

    let items = vec![
        ListItem::new("1", "设计稿")
            .subtitle("昨天更新")
            .meta("12 MB")
            .icon(IconName::IMAGE),
        ListItem::new("2", "组件清单")
            .subtitle("今天更新")
            .meta("38 KB")
            .icon(IconName::LIST)
            .chevron(),
        ListItem::new("3", "发布记录")
            .subtitle("3 天前")
            .meta("v0.1.0")
            .icon(IconName::CLOUD),
        ListItem::new("4", "素材库")
            .subtitle("只读")
            .meta("2.4 GB")
            .icon(IconName::IMAGES),
    ];

    let base_rows = vec![
        TableRow::new("1", vec!["Atlas", "设计", "12"]),
        TableRow::new("2", vec!["Borealis", "工程", "8"]),
        TableRow::new("3", vec!["Cinder", "运营", "21"]),
        TableRow::new("4", vec!["Delta", "设计", "5"]),
    ];
    let columns = vec![
        TableColumn::new("name", "名称").sortable(),
        TableColumn::new("team", "团队"),
        TableColumn::new("count", "数量").right(),
    ];

    /* ---------------------------------------------------- P3-C 数据类样例 */

    let table_columns = vec![
        DataColumn::new("name", "名称").width(180.0).sortable(),
        // 这一列故意把 min_width 设得比初始宽度还大：拖动时能直观看到兜底。
        DataColumn::new("kind", "类型").width(120.0).min_width(140.0),
        DataColumn::new("size", "大小 (KB)")
            .width(96.0)
            .right()
            .sortable(),
        DataColumn::new("modified", "修改时间").width(150.0),
        DataColumn::new("state", "状态").width(90.0),
    ];
    let table_rows: Vec<DataTableRow> = (1..=16)
        .map(|i| {
            let kind = ["设计", "文档", "图片"][i % 3];
            let state = ["已发布", "草稿", "归档"][i % 3];
            DataTableRow::new(
                format!("t{i}"),
                vec![
                    format!("素材 {i:02}"),
                    kind.to_string(),
                    format!("{}", 120 + i * 37),
                    format!(
                        "11-{:02} {:02}:{:02}",
                        i % 28 + 1,
                        9 + i % 9,
                        (i * 7) % 60
                    ),
                    state.to_string(),
                ],
            )
        })
        .collect();

    let virtual_columns = vec![
        DataColumn::new("no", "#").width(64.0).right(),
        DataColumn::new("name", "节点").width(200.0),
        DataColumn::new("cpu", "CPU %").width(96.0).right().sortable(),
        DataColumn::new("mem", "内存 (MB)").width(120.0).right(),
    ];
    let virtual_rows: Vec<DataTableRow> = (0..200)
        .map(|i| {
            DataTableRow::new(
                format!("v{i}"),
                vec![
                    format!("{}", i + 1),
                    format!("worker-{:03}", i + 1),
                    format!("{}", (i * 7) % 100),
                    format!("{}", 256 + i * 8),
                ],
            )
        })
        .collect();

    /* ------------------------------------------ 三件套：受控选中 / 服务端排序 / 行内编辑 */

    let edit_columns = vec![
        DataColumn::new("name", "名称").width(180.0).sortable().editable(),
        DataColumn::new("kind", "类型").width(110.0).editable(),
        DataColumn::new("size", "大小 (KB)").width(120.0).right().sortable().editable(),
    ];
    // 行数据在调用方：提交后由这里改写，组件只上报 CellEdit。
    let edit_rows = RwSignal::new(
        (1..=6)
            .map(|i| {
                let kind = ["设计", "文档", "图片"][i % 3];
                DataTableRow::new(
                    format!("e{i}"),
                    vec![
                        format!("素材 {i:02}"),
                        kind.to_string(),
                        // 三位数：演示里的字符串比较与数值比较同序，别把排序差异
                        // 误当成组件的 bug。
                        format!("{}", 120 + i * 6),
                    ],
                )
            })
            .collect::<Vec<_>>(),
    );
    let picked = RwSignal::new(Some(vec!["e2".to_string()]));
    // 「服务端」在这一页就是调用方自己排序；组件只上报 SortSpec、不再本地排。
    let server_sort = RwSignal::new(Some(SortSpec::new("name", SortDir::Asc)));

    let tree_selected = RwSignal::new("src".to_string());
    let tree_nodes = vec![
        TreeNode::new("root", "glass-design")
            .icon(IconName::FOLDER_OPEN)
            .child(
                TreeNode::new("src", "src")
                    .icon(IconName::FOLDER)
                    .child(
                        TreeNode::new("widgets", "widgets")
                            .icon(IconName::LAYERS)
                            .badge("12"),
                    )
                    .child(TreeNode::new("pages", "pages").icon(IconName::FOLDER)),
            )
            .child(
                TreeNode::new("assets", "assets")
                    .icon(IconName::FOLDER)
                    .disabled()
                    .child(TreeNode::new("logo", "logo.png").icon(IconName::IMAGE)),
            )
            .child(TreeNode::new("toml", "Cargo.toml").icon(IconName::FILE_TEXT))
            .child(
                TreeNode::new("remote", "远端缓存")
                    .icon(IconName::CLOUD)
                    .loading(),
            ),
        TreeNode::new("docs", "文档")
            .icon(IconName::FOLDER)
            .child(TreeNode::new("readme", "README.md").icon(IconName::FILE_TEXT))
            .child(TreeNode::new("spec", "规范").icon(IconName::FILE_TEXT))
            .child(TreeNode::new("archive", "归档").icon(IconName::FOLDER)),
    ];

    let grid_items: Vec<CollectionItem> = (1..=9)
        .map(|i| {
            let item = CollectionItem::new(format!("g{i}"), format!("资源 {i}"))
                .subtitle(format!("{} 项", i * 3));
            if i == 1 {
                item.selected()
            } else {
                item
            }
        })
        .collect();
    let list_items: Vec<CollectionItem> = (1..=5)
        .map(|i| {
            CollectionItem::new(format!("l{i}"), format!("序列 {i}"))
                .subtitle(format!("第 {i} 组"))
        })
        .collect();

    let multi_list_count = RwSignal::new(0usize);
    let multi_list_items: Vec<ListItem> = (1..=8)
        .map(|i| {
            ListItem::new(format!("m{i}"), format!("任务 {i:02}"))
                .subtitle(if i % 2 == 0 { "进行中" } else { "待处理" })
                .meta(format!("{i} h"))
        })
        .collect();

    /* ------------------------------------------ P4 万级数据：虚拟滚动样例 */

    // 3 层 × 循环生成：20 组 × 9 目录 × 10 文件 + 200 个父行 = 2000 个可见行。
    let big_tree_selected = RwSignal::new("g0".to_string());
    let big_tree: Vec<TreeNode> = (0..20)
        .map(|g| {
            TreeNode::new(format!("g{g}"), format!("模块 {g:02}"))
                .icon(IconName::FOLDER)
                .children(
                    (0..9)
                        .map(|d| {
                            TreeNode::new(format!("g{g}-d{d}"), format!("目录 {d:02}"))
                                .icon(IconName::FOLDER)
                                .children(
                                    (0..10)
                                        .map(|f| {
                                            TreeNode::new(
                                                format!("g{g}-d{d}-f{f}"),
                                                format!("文件 {f:02}.rs"),
                                            )
                                            .icon(IconName::FILE_TEXT)
                                        })
                                        .collect(),
                                )
                        })
                        .collect(),
                )
        })
        .collect();

    let big_items: Vec<CollectionItem> = (0..2000)
        .map(|i| {
            CollectionItem::new(format!("b{i}"), format!("资源 {i:04}"))
                .subtitle(format!("{} 项", (i * 7) % 40 + 1))
        })
        .collect();

    // 变高行：正文长度按 n % 4 递增（约 1–4 行），高度必须真的不一样，
    // 否则「实测校正」这件事看不出来。
    let var_items: Vec<CollectionItem> = (0..600)
        .map(|i| {
            let line = "行高不一的示例正文，用来看清窗口校正。";
            let body = line.repeat((i % 4 + 1) * 4);
            CollectionItem::new(format!("h{i}"), format!("变高项 {i:03}")).subtitle(body)
        })
        .collect();

    view! {
        <div class="l-scroll" style="padding:24px 32px 64px">
            <div class="l-stack l-stack--6">
                <Breadcrumb items=vec![
                    Crumb::new("工作区").icon(IconName::HOUSE),
                    Crumb::new("资料库").icon(IconName::LAYERS),
                    Crumb::new("组件清单"),
                ] />

                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"数据"</h1>
                    <p class="c-subtitle" style="white-space:normal">"列表、表格、分页。选择与排序状态由调用方持有。"</p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"格子"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--4">
                            <Tile media_icon=IconName::IMAGE title="设计稿" subtitle="12 项" selected=true />
                            <Tile media_icon=IconName::IMAGES title="素材库" subtitle="2.4 GB" badge="新" />
                            <Tile media_icon=IconName::FILE_TEXT title="文档" subtitle="38 个" />
                            <Tile media_icon=IconName::CLOUD title="归档" subtitle="只读" />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"列表"</SectionTitle>
                    <Card>
                        <List items=items
                            selected=Signal::derive(move || selected.get())
                            on_select=Callback::new(move |k| selected.set(Some(k)))
                            on_activate=Callback::new(move |k| { toast_with(format!("打开 {k}"), Tone::Info, None); }) />
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"表格"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            {move || {
                                let mut rows = base_rows.clone();
                                if let Some(s) = sort.get() {
                                    rows.sort_by(|a, b| {
                                        let ai = columns.iter().position(|c| c.key == s.key).unwrap_or(0);
                                        let bi = ai;
                                        let cmp = a.cells.get(ai).cmp(&b.cells.get(bi));
                                        if s.dir == SortDir::Asc { cmp } else { cmp.reverse() }
                                    });
                                }
                                view! {
                                    <Table columns=columns.clone() rows=rows
                                        sort=sort.get()
                                        on_sort=Callback::new(move |(key, dir)| sort.set(Some(TableSort { key, dir })))
                                        selected_key=selected.get()
                                        on_select=Callback::new(move |k| selected.set(Some(k))) />
                                }
                            }}
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"分页"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <Pagination page=Signal::derive(move || page.get()) total_pages=8
                                on_change=Callback::new(move |p| page.set(p)) />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"数据表"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <div class="l-stack l-stack--6">
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">"5 列 × 16 行：名称 / 大小可排序，类型列 140px 下限，多选（Ctrl/Cmd 切换、Shift 连选、Ctrl+A 全选）；↑/↓ 只移焦点，Space 切换选中。"</p>
                                    <div style="width:720px;height:320px;display:grid">
                                        <DataTable
                                            columns=table_columns
                                            rows=table_rows
                                            selection=SelectionMode::Multiple
                                            default_selected=vec!["t2".to_string()]
                                            label="素材"
                                        />
                                    </div>
                                </div>
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">"虚拟行：200 行，固定行高 36px；滚动时只渲染窗口 ± 6 行。"</p>
                                    <div style="width:720px;height:320px;display:grid">
                                        <DataTable
                                            columns=virtual_columns
                                            rows=virtual_rows
                                            virtual_row_height=36.0
                                            label="节点"
                                        />
                                    </div>
                                </div>
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">"三件套：选中受控（选中集在调用方，下面那行计数走的是调用方的值）；排序服务端（组件只报 SortSpec，行序由调用方给）；双击单元格或焦点行按 Enter / F2 编辑，Enter 或失焦提交、Esc 取消，名称不能为空、大小必须是数字。"</p>
                                    <div style="width:720px;height:260px;display:grid">
                                        {move || {
                                            // 这里就是「服务端」：拿 SortSpec 自己排，组件不再本地排。
                                            let mut rows = edit_rows.get();
                                            if let Some(spec) = server_sort.get() {
                                                let ci = edit_ci(&spec.key);
                                                rows.sort_by(|a, b| {
                                                    let ord = a.cells.get(ci).cmp(&b.cells.get(ci));
                                                    if spec.dir == SortDir::Asc { ord } else { ord.reverse() }
                                                });
                                            }
                                            view! {
                                                <DataTable
                                                    columns=edit_columns.clone()
                                                    rows=rows
                                                    selection=SelectionMode::Multiple
                                                    selected=picked
                                                    on_selection_change=Callback::new(move |v| picked.set(Some(v)))
                                                    sort=server_sort
                                                    on_sort_change=Callback::new(move |s| server_sort.set(Some(s)))
                                                    on_cell_commit=Callback::new(move |edit: CellEdit| {
                                                        edit_rows.update(|rows| {
                                                            if let Some(row) = rows.iter_mut().find(|r| r.id == edit.row_id) {
                                                                if let Some(cell) = row.cells.get_mut(edit_ci(&edit.key)) {
                                                                    *cell = edit.value;
                                                                }
                                                            }
                                                        });
                                                    })
                                                    validate=Callback::new(move |edit: CellEdit| -> Option<String> {
                                                        match edit.key.as_str() {
                                                            "name" if edit.value.trim().is_empty() => Some("名称不能为空".to_string()),
                                                            "size" if edit.value.trim().parse::<f64>().is_err() => Some("必须是数字".to_string()),
                                                            _ => None,
                                                        }
                                                    })
                                                    label="可编辑素材"
                                                />
                                            }
                                        }}
                                    </div>
                                    <p class="c-subtitle">
                                        {move || {
                                            format!(
                                                "调用方持有选中集：已选 {} 项；当前排序 {:?}。",
                                                picked.get().map(|v| v.len()).unwrap_or(0),
                                                server_sort.get(),
                                            )
                                        }}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"树"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <TreeView
                                nodes=tree_nodes
                                default_open=true
                                selected=Signal::derive(move || tree_selected.get())
                                on_select=Callback::new(move |k| tree_selected.set(k))
                                on_retry=Callback::new(move |k| {
                                    toast_with(format!("重试 {k}"), Tone::Info, None);
                                })
                                label="文件树"
                            />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"集合"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <div class="l-stack l-stack--6">
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">"网格 3 列 × 9 项，多选。"</p>
                                    <CollectionView
                                        items=grid_items
                                        layout=CollectionLayout::Grid
                                        columns=3
                                        selection=SelectionMode::Multiple
                                        label="资源"
                                    />
                                </div>
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">"列表形态 5 项，单选。"</p>
                                    <CollectionView
                                        items=list_items
                                        layout=CollectionLayout::List
                                        label="序列"
                                    />
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"万级数据（虚拟滚动）"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <div class="l-stack l-stack--6">
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">
                                        "树：2000 行（20 组 × 9 目录 × 10 文件），视口 320px、行高 26px；只渲染窗口 ± 6 行，↑↓←→ / Home / End / Enter 可用。"
                                    </p>
                                    <TreeView
                                        nodes=big_tree
                                        default_open=true
                                        max_height=320.0
                                        virtual_row_height=26.0
                                        selected=Signal::derive(move || big_tree_selected.get())
                                        on_select=Callback::new(move |k| big_tree_selected.set(k))
                                        label="大文件树"
                                    />
                                </div>
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">
                                        "网格：2000 项 / 3 列 / 667 行，视口 360px、行距 52px（项高 48 + 间距 4）；只渲染窗口 ± 6 行，Ctrl / Shift 与方向键按全量下标算。"
                                    </p>
                                    <CollectionView
                                        items=big_items
                                        layout=CollectionLayout::Grid
                                        columns=3
                                        selection=SelectionMode::Multiple
                                        max_height=360.0
                                        virtual_row_height=52.0
                                        label="大资源网格"
                                    />
                                </div>
                                <div class="l-stack l-stack--3">
                                    <p class="c-subtitle">
                                        "变高行：600 项，每项正文约 1–4 行（第 n 项按 n % 4 递增），视口 420px。先按估计高度排、量到真高后校正；DOM 里只有窗口量级的项，↑↓ 先滚进视口再聚焦。"
                                    </p>
                                    <CollectionView
                                        items=var_items
                                        layout=CollectionLayout::List
                                        max_height=420.0
                                        virtual_var_height=true
                                        label="变高列表"
                                    />
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"列表多选"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <div class="l-stack l-stack--3">
                                <p class="c-subtitle">
                                    {move || format!("已选 {} 项", multi_list_count.get())}
                                </p>
                                <List
                                    items=multi_list_items
                                    selection=ListSelection::Multiple
                                    on_selection_change=Callback::new(move |ids: Vec<String>| {
                                        multi_list_count.set(ids.len());
                                    })
                                />
                            </div>
                        </div>
                    </Card>
                </div>

            </div>
        </div>
    }
}

/// 「可编辑素材」那三列的 cells 下标固定（name / kind / size）。
/// 一页演示不值得再建一层「列 key → 下标」的映射表，写死在调用方最直白。
fn edit_ci(key: &str) -> usize {
    match key {
        "kind" => 1,
        "size" => 2,
        _ => 0,
    }
}
