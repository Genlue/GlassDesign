use glass_design::prelude::*;
use leptos::prelude::*;

/// 面板分区：检查器 / 大纲 / 分栏 / 状态栏。
///
/// 四块都用「值受控、点击只上报」的写法 —— 页面持有唯一真相，
/// 组件只负责把状态翻译成类名与 `data-*`。
#[component]
pub fn PanelsPage() -> impl IntoView {
    // —— 检查器 ——
    let tab = RwSignal::new("属性".to_string());
    let visible = RwSignal::new(true);
    let opacity = RwSignal::new(0.62);
    let align = RwSignal::new("center".to_string());

    // —— 大纲 ——
    let outline_at = RwSignal::new("hero".to_string());

    let tabs = vec!["属性".to_string(), "外观".to_string()];
    // 用闭包而不是 Vec：页签内容是响应式闭包，反复调用时每次都要一份新的选项。
    let aligns = || {
        vec![
            SelectOption::new("left", "左对齐"),
            SelectOption::new("center", "水平居中"),
            SelectOption::new("right", "右对齐"),
        ]
    };

    // 8 条、三级缩进。depth 从 0 起，组件内部 +1 写成 aria-level / data-level。
    let outline = vec![
        OutlineItem::new("cover", "封面", 0).icon(IconName::FILE_TEXT),
        OutlineItem::new("title", "标题", 1),
        OutlineItem::new("subtitle", "副标题", 1),
        OutlineItem::new("hero", "主视觉", 0).icon(IconName::IMAGE),
        OutlineItem::new("glow", "光斑", 1),
        OutlineItem::new("noise", "颗粒纹理", 2),
        OutlineItem::new("body", "说明文字", 0).icon(IconName::FILE_TEXT),
        OutlineItem::new("footer", "页脚", 1),
    ];

    // SplitView 用两个 Children 槽（Leptos 的 Children 是 FnOnce，一份拆不成两栏）。
    // 说明文字要 white-space:normal：.c-subtitle 是为顶栏设计的 nowrap，
    // 放进 30% 宽的栏里会被硬裁掉半句。
    let split_left: Children = ToChildren::to_children(|| {
        view! {
            <div class="l-pad l-stack l-stack--3">
                <div class="c-subtitle">"左栏 · 30%"</div>
                <p class="c-subtitle" style="white-space:normal">
                    "拖分隔条改比例，双击复位；分隔条聚焦后 ←/→ 步进 2%（Shift 10%），Home / End 到上下限。"
                </p>
            </div>
        }
    });
    let split_right: Children = ToChildren::to_children(|| {
        view! {
            <div class="l-pad">
                <Card>
                    <div class="l-pad l-stack l-stack--2">
                        <div class="c-subtitle">"右栏"</div>
                        <p class="c-subtitle" style="white-space:normal">
                            "两块内容由调用方给，组件只管比例与折叠。"
                        </p>
                    </div>
                </Card>
            </div>
        }
    });

    let v_left: Children = ToChildren::to_children(|| {
        view! {
            <div class="l-pad l-stack l-stack--2">
                <div class="c-subtitle">"上栏 · 40%"</div>
                <p class="c-subtitle" style="white-space:normal">"纵向分栏用 ↑/↓ 改比例，其余规则相同。"</p>
            </div>
        }
    });
    let v_right: Children = ToChildren::to_children(|| {
        view! {
            <div class="l-pad">
                <Card>
                    <div class="l-pad c-subtitle">"下栏"</div>
                </Card>
            </div>
        }
    });

    // 状态栏右侧槽同样是 Children。
    let status_right: Children = ToChildren::to_children(|| {
        view! {
            // value 是 0..1 的比例，不是百分数：和旧库 status-bar 的口径一致。
            <StatusProgress label="导出 PDF" value=0.72 phase="running" />
        }
    });

    view! {
        <div class="gd-page">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"面板"</h1>
                    <p class="c-subtitle" style="white-space:normal">
                        "检查器、大纲、分栏与状态栏。面板高度都不定，所以玻璃档位只单独展示，不绑进组件。"
                    </p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"检查器"</SectionTitle>
                    <p class="c-subtitle" style="white-space:normal">
                        "外壳只做「页签 + 一页内容」。Children 是 FnOnce，拆不成多页，
                        所以页签只上报，内容由调用方按 selected_tab 自己换。"
                    </p>
                    <div style="width:320px;height:420px">
                        <Inspector
                            title="检查器"
                            tabs=tabs
                            selected_tab=Signal::derive(move || tab.get())
                            on_tab=Callback::new(move |v| tab.set(v))
                        >
                            // 页签内容用一个响应式闭包换，不用 Show：
                            // Show 的 children 要求 Fn，闭包里再 move 出 Vec 会退化成 FnOnce。
                            {move || {
                                if tab.get() == "属性" {
                                    view! {
                                        <PropList title="内容">
                                            <PropRow label="名称">
                                                <span>"矩形 12"</span>
                                            </PropRow>
                                            <PropRow label="可见">
                                                <Switch
                                                    label="可见"
                                                    checked=Signal::derive(move || visible.get())
                                                    on_change=Callback::new(move |v| visible.set(v))
                                                />
                                            </PropRow>
                                            <PropRow label="不透明度">
                                                // 控件宽度由调用方定：值区只负责靠右排。
                                                <div style="width:96px">
                                                    <Slider
                                                        label="不透明度"
                                                        min=0.0
                                                        max=1.0
                                                        step=0.01
                                                        value=Signal::derive(move || opacity.get())
                                                        on_input=Callback::new(move |v| opacity.set(v))
                                                        on_change=Callback::new(move |v| opacity.set(v))
                                                    />
                                                </div>
                                            </PropRow>
                                        </PropList>
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <PropList title="外观">
                                            <PropRow label="宽度">
                                                <span>"240 px"</span>
                                            </PropRow>
                                            <PropRow label="对齐">
                                                <Select
                                                    width=104
                                                    // 每次重建都新建一份选项：这个闭包要被反复调用（Fn），
                                                    // 捕获一个 Vec 进去就只能调一次。
                                                    options=aligns()
                                                    value=Signal::derive(move || align.get())
                                                    on_change=Callback::new(move |v| align.set(v))
                                                />
                                            </PropRow>
                                            <PropRow label="圆角">
                                                <span>"12 px"</span>
                                            </PropRow>
                                        </PropList>
                                    }
                                    .into_any()
                                }
                            }}
                        </Inspector>
                    </div>

                    <div class="l-stack l-stack--2" style="align-items:flex-start">
                        // GlassPanel 的 children 是必填的（组件不收可选子节点），
                        // 这里给一个空 span：面板本身固定 480×320，子节点不影响贴图。
                        <GlassPanel size=GlassPanelSize::P480x320><span></span></GlassPanel>
                        <p class="c-subtitle" style="white-space:normal">
                            "玻璃面板档位（480×320）：贴图按这个尺寸预计算，边缘有折射与色散。
                            检查器自己不定高，所以它走 Auto（只有涂层，无折射）—— 这才是正确的用法。"
                        </p>
                    </div>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"大纲"</SectionTitle>
                    <p class="c-subtitle" style="white-space:normal">
                        "扁平 DOM + aria-level，缩进写 --depth；↑/↓/Home/End 移动焦点并改选中，Enter 再确认一次。"
                    </p>
                    <Card>
                        <div class="l-pad">
                            <Outline
                                items=outline
                                selected=Signal::derive(move || outline_at.get())
                                on_select=Callback::new(move |v| outline_at.set(v))
                            />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"分栏"</SectionTitle>
                    <p class="c-subtitle" style="white-space:normal">
                        "只支持两栏：嵌套两次就是三栏。比例由 CSS 算（--split），渲染期不测量容器。"
                    </p>
                    <div style="height:200px">
                        <SplitView
                            initial=0.3
                            min=0.2
                            max=0.7
                            collapsible=true
                            first=split_left
                            second=split_right
                        />
                    </div>
                    <div style="height:240px">
                        <SplitView
                            orientation=SplitOrientation::Vertical
                            initial=0.4
                            min=0.2
                            max=0.8
                            first=v_left
                            second=v_right
                        />
                    </div>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"状态栏"</SectionTitle>
                    <p class="c-subtitle" style="white-space:normal">
                        "贴边长期占位，所以是内容面不是玻璃面：只有 --surface-content 加一条上边发丝。"
                    </p>
                    <Card>
                        <StatusBar right=status_right>
                            <StatusItem
                                icon=IconName::CLOUD
                                label="同步"
                                value="刚刚"
                                live=StatusTone::Success
                            />
                            <StatusItem icon=IconName::HARD_DRIVE label="缓存" value="182 MB" />
                            <StatusItem
                                icon=IconName::MOUSE_POINTER
                                label="光标"
                                value="1240, 680"
                                on_click=Callback::new(move |_| {
                                    toast("已复制光标位置");
                                })
                            />
                        </StatusBar>
                    </Card>
                </div>
            </div>
        </div>
    }
}
