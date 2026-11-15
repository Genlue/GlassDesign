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
                    <p class="c-subtitle">"列表、表格、分页。选择与排序状态由调用方持有。"</p>
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
            </div>
        </div>
    }
}
