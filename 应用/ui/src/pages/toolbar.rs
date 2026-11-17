use glass_design::prelude::*;
use leptos::prelude::*;

/// 工具栏分区：溢出收纳 + 自定义。
///
/// 两个示例刻意分开摆：溢出是「宽度说了算」的自动行为，自定义是「用户说了算」的
/// 显式设置。把它们混在一张卡里，看的人会以为是同一个东西的两种状态。
#[component]
pub fn ToolbarPage() -> impl IntoView {
    /* ---------------- 溢出收纳 ---------------- */

    // 容器宽度三档：窄 / 中 / 宽。用 max-width 是为了让「窗口变宽」也能验证同一件事。
    let width = RwSignal::new("260".to_string());
    let width_options = vec![
        SegmentedOption::new("260", "窄"),
        SegmentedOption::new("340", "中"),
        SegmentedOption::new("420", "宽"),
    ];

    let actions = vec![
        ToolbarAction::new("bold", "加粗", IconName::FILE_TEXT),
        ToolbarAction::new("italic", "斜体", IconName::SQUARE_PEN),
        ToolbarAction::new("underline", "下划线", IconName::MINUS),
        ToolbarAction::new("align", "对齐", IconName::COLUMNS_2),
        ToolbarAction::new("list", "列表", IconName::LIST),
        ToolbarAction::new("link", "链接", IconName::EXTERNAL_LINK),
        ToolbarAction::new("image", "图片", IconName::IMAGE),
        ToolbarAction::new("table", "表格", IconName::GRID_2X2),
        ToolbarAction::new("code", "代码", IconName::SQUARE_DASHED),
        ToolbarAction::new("preview", "预览", IconName::EYE),
    ];
    // 收纳顺序：预览 / 代码 / 表格先走，其余从右往左收。
    let priority = vec![
        "preview".to_string(),
        "code".to_string(),
        "table".to_string(),
    ];
    let labels = actions.clone();
    let last_action = RwSignal::new("—".to_string());
    let on_action = Callback::new(move |id: String| {
        let label = labels
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.label.clone())
            .unwrap_or(id);
        last_action.set(label.clone());
        toast(format!("动作：{label}"));
    });

    /* ---------------- 自定义 ---------------- */

    // 8 行，含 1 个锁定项（分享）与 2 个默认关掉的项。
    let fields = vec![
        ToolbarField::new("undo", "撤销"),
        ToolbarField::new("redo", "重做"),
        ToolbarField::new("format", "格式刷"),
        ToolbarField::new("align", "对齐").hidden(),
        ToolbarField::new("comment", "批注").hidden(),
        ToolbarField::new("share", "分享").locked(),
        ToolbarField::new("zoom", "缩放"),
        ToolbarField::new("fullscreen", "全屏"),
    ];
    let total = fields.len();
    // 组件自己维护顺序与显隐，这里只镜像一份用来显示「当前顺序 / 可见数」。
    // 首屏的值由组件挂载后回报（含 localStorage 恢复的结果）。
    let current = RwSignal::new(fields.clone());

    view! {
        <div class="gd-page">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"工具栏"</h1>
                    <p class="c-subtitle" style="white-space:normal">
                        "放不下的动作收进「更多」；放哪些、什么顺序，则由用户自己定。"
                    </p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"溢出收纳"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <div class="l-row l-row--3" style="flex-wrap:wrap">
                                <Segmented
                                    options=width_options
                                    value=Signal::derive(move || width.get())
                                    on_change=Callback::new(move |v| width.set(v))
                                />
                                <span class="c-subtitle" style="white-space:normal">
                                    "改容器宽度看收纳变化；「⋯」打开被收起的项，Esc 关闭。"
                                </span>
                            </div>

                            // .c-toolbar 是真实的顶栏样式（52 高 + 磨砂），
                            // 用它当宿主才像一条工具栏，而不是一排悬空的图标。
                            <div
                                class="c-toolbar"
                                style=move || {
                                    let w = width.get().parse::<f64>().unwrap_or(420.0);
                                    format!("max-width:{w}px")
                                }
                            >
                                <ToolbarOverflow
                                    actions=actions.clone()
                                    priority=priority.clone()
                                    label="格式工具栏"
                                    on_action=on_action
                                />
                            </div>

                            <div class="c-subtitle" style="white-space:normal">
                                "最近的动作：" {move || last_action.get()}
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"自定义"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--6" style="align-items:flex-start">
                            // 固定 360：真实设置面板就是这个量级。放到 800+ 宽时，
                            // 「名称在左、开关在右」中间会空出半个屏幕，行读不成行。
                            <div style="width:360px;flex:none">
                                <ToolbarCustomization
                                    fields=fields.clone()
                                    storage_key="gd.demo.toolbar".to_string()
                                    on_change=Callback::new(move |list: Vec<ToolbarField>| {
                                        current.set(list);
                                    })
                                />
                            </div>

                            <div class="l-fill l-stack l-stack--3">
                                <div class="c-subtitle" style="white-space:normal">
                                    "可见 "
                                    {move || current.get().iter().filter(|f| f.visible).count()}
                                    " / "
                                    {total}
                                    "　·　顺序与显隐存进 localStorage"
                                </div>

                                // 预览条：按当前顺序把可见项画成一排小胶囊，
                                // 这样「拖完到底变成什么样」不用去读文字。
                                <div class="l-row l-row--2" style="flex-wrap:wrap">
                                    {move || {
                                        current
                                            .get()
                                            .into_iter()
                                            .filter(|f| f.visible)
                                            .map(|f| {
                                                view! { <Chip label=f.label size=Size::Sm /> }
                                            })
                                            .collect_view()
                                    }}
                                </div>

                                <div class="c-subtitle" style="white-space:normal">
                                    {move || {
                                        let order = current
                                            .get()
                                            .into_iter()
                                            .map(|f| f.label)
                                            .collect::<Vec<String>>()
                                            .join(" · ");
                                        format!("当前顺序：{order}")
                                    }}
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}
