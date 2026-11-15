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
            </div>
        </div>
    }
}
