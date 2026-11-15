use glass_design::prelude::*;
use leptos::prelude::*;

#[component]
pub fn NavPage() -> impl IntoView {
    let tab = RwSignal::new("details".to_string());
    let closed_tab = RwSignal::new("a".to_string());
    let tabs = vec![
        TabItem::new("details", "详情").icon(IconName::FILE_TEXT),
        TabItem::new("activity", "活动").icon(IconName::COMPASS),
        TabItem::new("settings", "设置").icon(IconName::SETTINGS),
    ];

    // —— 停靠栏 ——
    let dock_value = RwSignal::new("home".to_string());
    let dock_items = vec![
        DockItem::new("home", "工作台", IconName::HOUSE),
        DockItem::new("search", "搜索", IconName::SEARCH).badge("3"),
        DockItem::new("library", "素材库", IconName::IMAGES),
        DockItem::new("star", "收藏", IconName::STAR),
        DockItem::new("layers", "图层", IconName::LAYERS),
        DockItem::new("settings", "设置", IconName::SETTINGS),
        DockItem::new("cloud", "云同步", IconName::CLOUD).badge("!"),
    ];
    let dock_for_label = dock_items.clone();
    let dock_label = move |id: &str| {
        dock_for_label
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.label.clone())
            .unwrap_or_default()
    };

    // —— 文档页签 ——
    let doc_active = RwSignal::new("readme".to_string());
    let doc_list = RwSignal::new(vec![
        DocumentTab::new("index", "index.html").pinned(),
        DocumentTab::new("readme", "readme.md").dirty(),
        DocumentTab::new("app", "app.rs").dirty(),
        DocumentTab::new("changes", "变更.md").closable(false),
        DocumentTab::new("theme", "theme.css"),
        DocumentTab::new("notes", "notes.txt"),
    ]);

    // —— 导航搜索 ——
    let query = RwSignal::new(String::new());

    view! {
        <div class="l-scroll" style="padding:24px 32px 64px">
            <div class="l-stack l-stack--6">
                <Breadcrumb items=vec![
                    Crumb::new("工作区").icon(IconName::HOUSE),
                    Crumb::new("项目").icon(IconName::LAYERS),
                    Crumb::new("Atlas"),
                ] />

                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"导航"</h1>
                    <p class="c-subtitle">"页签、面包屑、分页；键盘方向键切换。"</p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"信息卡"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--3">
                            <NavInfo icon=IconName::SETTINGS title="参数" subtitle="应用设置" />
                            <NavInfo title="小 Q" subtitle="本地账户" on_click=Callback::new(move |_| { toast("打开账户"); }) />
                            <NavCard solid=true icon=IconName::LAYERS title="组件清单" desc="51 个组件，五个分区" />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"页签"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Tabs tabs=tabs value=Signal::derive(move || tab.get())
                                on_change=Callback::new(move |v| tab.set(v)) />
                            <div class="c-subtitle">
                                {move || match tab.get().as_str() {
                                    "details" => "详情面板：名称、负责人、更新时间。",
                                    "activity" => "活动面板：最近的提交与评论。",
                                    _ => "设置面板：可见性与通知。",
                                }}
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"可关闭页签"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <Tabs tabs=vec![
                                    TabItem::new("a", "设计稿"),
                                    TabItem::new("b", "组件清单"),
                                    TabItem::new("c", "发布记录"),
                                ]
                                value=Signal::derive(move || closed_tab.get())
                                on_change=Callback::new(move |v| closed_tab.set(v))
                                closable=true
                                on_close=Callback::new(move |v| { toast(format!("关闭 {v}")); }) />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"工具栏"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3">
                            <Button label="主要" variant=ButtonVariant::Primary />
                            <Button label="次要" />
                            <Capsule size=CapsuleSize::Md>
                                <span style="font-size:13px">"玻璃胶囊"</span>
                            </Capsule>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"停靠栏"</SectionTitle>
                    <Card>
                        // .c-dock 是绝对定位（设计上贴在应用边缘），
                        // 放进流式卡片得给它一个自己的定位上下文
                        <div class="l-pad" style="position:relative;min-height:400px">
                            <div
                                class="l-stack l-stack--3"
                                style="padding-left:76px;position:absolute;top:50%;transform:translateY(-50%)"
                            >
                                <div class="c-subtitle">
                                    "当前项："
                                    {move || dock_label(&dock_value.get())}
                                </div>
                                <div class="c-subtitle">
                                    "↑ / ↓ 在圆钮间移动焦点，Enter 或 Space 选中；第三颗后面是分隔线，第二、七颗带角标。"
                                </div>
                            </div>
                            <NavDock
                                items=dock_items.clone()
                                value=Signal::derive(move || dock_value.get())
                                on_select=Callback::new(move |id| dock_value.set(id))
                                orientation=DockOrientation::Left
                                label="示例停靠栏"
                                sep_after="library"
                            />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"文档页签"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--3" style="width:640px">
                            <DocumentTabs
                                // 首屏快照：关闭/重排由组件自己维护，页面只镜像一份用来提示
                                tabs=doc_list.get_untracked()
                                value=Signal::derive(move || doc_active.get())
                                on_select=Callback::new(move |id| doc_active.set(id))
                                on_close=Callback::new(move |id: String| {
                                    let label = doc_list.with_untracked(|list| {
                                        list.iter()
                                            .find(|t| t.id == id)
                                            .map(|t| t.label.clone())
                                            .unwrap_or_else(|| id.clone())
                                    });
                                    doc_list.update(|list| list.retain(|t| t.id != id));
                                    toast(format!("关闭 {label}"));
                                })
                                on_new=Callback::new(move |_| { toast("新建文档"); })
                                on_reorder=Callback::new(move |ids: Vec<String>| {
                                    // 组件自己已经排好了；这里同步镜像，
                                    // 免得切走再切回来时顺序又退回初始快照。
                                    doc_list.update(|list| {
                                        let mut next = Vec::with_capacity(list.len());
                                        for id in &ids {
                                            if let Some(pos) = list.iter().position(|t| &t.id == id) {
                                                next.push(list[pos].clone());
                                            }
                                        }
                                        *list = next;
                                    });
                                    toast(format!("重排：{}", ids.join(" · ")));
                                })
                            />
                            <div class="c-subtitle">
                                "← / → 切换，Home / End 首尾，Ctrl+W 关当前，Alt+← / → 与相邻页签换位；按住一颗横向拖过 5px 可以重排。"
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"导航搜索"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--3">
                            <NavSearch
                                value=Signal::derive(move || query.get())
                                placeholder="搜索全部内容"
                                shortcut="Ctrl+K"
                                on_input=Callback::new(move |v| query.set(v))
                                on_submit=Callback::new(move |v| { toast(format!("搜索：{v}")); })
                            />
                            <div class="c-subtitle">
                                "当前输入："
                                {move || {
                                    let q = query.get();
                                    if q.is_empty() { "（空）".to_string() } else { q }
                                }}
                                "　·　Ctrl+K 聚焦，Esc 清空并失焦"
                            </div>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}
