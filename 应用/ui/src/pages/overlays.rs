use glass_design::prelude::*;
use leptos::prelude::*;

fn recent_items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("recent-design", "设计稿.gd").icon(IconName::LAYERS),
        MenuItem::new("recent-assets", "素材库.gd").icon(IconName::IMAGES),
        MenuItem::new("recent-archive", "归档").icon(IconName::CLOUD).disabled(),
    ]
}

/// 8 项：分隔、勾选、快捷键、禁用、危险项、二级子菜单都覆盖到。
fn file_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::new("new", "新建文件")
            .icon(IconName::PLUS)
            .shortcut("Ctrl N"),
        MenuItem::new("open", "打开…")
            .icon(IconName::FOLDER_OPEN)
            .shortcut("Ctrl O"),
        MenuItem::new("recent", "最近打开")
            .icon(IconName::CLOCK)
            .children(recent_items()),
        MenuItem::new("save", "保存")
            .icon(IconName::HARD_DRIVE)
            .shortcut("Ctrl S")
            .separator_before(),
        MenuItem::new("autosave", "自动保存").checked(true),
        MenuItem::new("lock", "锁定编辑").icon(IconName::LOCK).disabled(),
        MenuItem::new("share", "分享链接")
            .icon(IconName::EXTERNAL_LINK)
            .separator_before(),
        MenuItem::new("delete", "删除项目")
            .icon(IconName::TRASH)
            .danger()
            .separator_before(),
    ]
}

fn long_menu() -> Vec<MenuItem> {
    (1..=12)
        .map(|i| MenuItem::new(format!("cmd-{i}"), format!("命令 {i}")))
        .collect()
}

#[component]
pub fn OverlayPage() -> impl IntoView {
    let show = RwSignal::new(false);
    let plan = RwSignal::new("pro".to_string());
    let options = vec![
        SelectOption::new("free", "免费版"),
        SelectOption::new("pro", "专业版"),
        SelectOption::new("team", "团队版"),
    ];

    // 浮层族的状态
    let picked = RwSignal::new(String::new());
    let pop_bottom = RwSignal::new(false);
    let pop_right = RwSignal::new(false);
    let sheet_open = RwSignal::new(false);
    let drawer_open = RwSignal::new(false);
    let tf_pop = RwSignal::new(false);
    // 气泡里放下拉：验证「父链」（下拉把气泡当父层，收起下拉不收气泡）。
    let pop_inner = RwSignal::new(false);
    let pop_inner_plan = RwSignal::new("pro".to_string());

    let on_pick = Callback::new(move |id: String| {
        picked.set(id.clone());
        toast(format!("选中：{id}"));
    });

    view! {
        <div class="gd-page">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"浮层"</h1>
                    <p class="c-subtitle" style="white-space:normal">"对话框、菜单、气泡、提示、流程面板与抽屉。Esc / 遮罩 / 点外关闭，层级与焦点统一交给浮层栈。"</p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"对话框"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3">
                            <Button label="打开对话框" variant=ButtonVariant::Primary
                                on_click=Callback::new(move |_| show.set(true)) />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"下拉"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3" style="align-items:center">
                            <Select options=options value=Signal::derive(move || plan.get())
                                on_change=Callback::new(move |v| plan.set(v)) />
                            <span class="c-subtitle">{move || format!("当前：{}", plan.get())}</span>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"提示条"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3">
                            <Button label="信息" on_click=Callback::new(move |_| { toast("这是一条提示"); }) />
                            <Button label="成功" on_click=Callback::new(move |_| { toast_with("保存成功", Tone::Success, None); }) />
                            <Button label="警告" on_click=Callback::new(move |_| { toast_with("需要注意", Tone::Warning, None); }) />
                            <Button label="错误" on_click=Callback::new(move |_| { toast_with("操作失败", Tone::Danger, None); }) />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"菜单"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--3">
                            <div class="l-row l-row--3" style="align-items:center">
                                <MenuButton label="文件" icon=IconName::FILE_TEXT
                                    items=file_menu() on_select=on_pick />
                                <MenuButton label="玻璃档 240×280" items=long_menu()
                                    fixed_size=true on_select=on_pick />
                                <span class="c-subtitle">
                                    {move || if picked.get().is_empty() {
                                        "↑↓ 循环、Home/End、→ 进子菜单、← 退回、字母定位、Esc 只关一层".to_string()
                                    } else {
                                        format!("最后选中：{}", picked.get())
                                    }}
                                </span>
                            </div>
                            <span class="c-subtitle">
                                "第二个按钮套 data-glass=\"menu-240x280\"：宽 240、高 280，项多了在菜单内部滚动。"
                            </span>
                            <span class="c-subtitle">
                                "同 rank 单开：开着「文件」再去开另一个菜单，「文件」自动收起、焦点跟到新菜单；"
                                "而「文件 → 最近打开」这种子菜单不会把父菜单收掉（靠 lineage 区分父子与无关浮层）。"
                            </span>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"气泡"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--3">
                            <div class="l-row l-row--3" style="align-items:center">
                                <Popover open=Signal::derive(move || pop_bottom.get())
                                    placement=PopoverPlacement::BottomStart
                                    on_close=Callback::new(move |_| pop_bottom.set(false))
                                    label="底部气泡"
                                    trigger=Box::new(move || view! {
                                        <Button label="底部气泡" on_click=Callback::new(move |_| pop_bottom.set(true)) />
                                    }.into_any())>
                                    <div class="l-stack l-stack--2">
                                        <span class="c-subtitle">"下方展开，放不下自动翻上。"</span>
                                        <div class="l-row l-row--3">
                                            <Button label="知道了" variant=ButtonVariant::Plain
                                                on_click=Callback::new(move |_| pop_bottom.set(false)) />
                                        </div>
                                    </div>
                                </Popover>
                                <Popover open=Signal::derive(move || pop_right.get())
                                    placement=PopoverPlacement::RightStart
                                    width=240
                                    on_close=Callback::new(move |_| pop_right.set(false))
                                    label="右侧气泡"
                                    trigger=Box::new(move || view! {
                                        <Button label="右侧气泡" on_click=Callback::new(move |_| pop_right.set(true)) />
                                    }.into_any())>
                                    <div class="l-stack l-stack--2">
                                        <span class="c-subtitle">"右侧贴边，放不下翻左。"</span>
                                    </div>
                                </Popover>
                                <Popover open=Signal::derive(move || pop_inner.get())
                                    placement=PopoverPlacement::BottomStart
                                    width=260
                                    on_close=Callback::new(move |_| pop_inner.set(false))
                                    label="气泡里放下拉"
                                    trigger=Box::new(move || view! {
                                        <Button label="气泡里放下拉" on_click=Callback::new(move |_| pop_inner.set(true)) />
                                    }.into_any())>
                                    <div class="l-stack l-stack--2">
                                        <span class="c-subtitle" style="white-space:normal">
                                            "下拉把气泡当父层：开下拉不收气泡，选完一项也不收（父链，不是点外）。"
                                        </span>
                                        <Select
                                            options=vec![
                                                SelectOption::new("free", "免费版"),
                                                SelectOption::new("pro", "专业版"),
                                                SelectOption::new("team", "团队版"),
                                            ]
                                            value=Signal::derive(move || pop_inner_plan.get())
                                            on_change=Callback::new(move |v| pop_inner_plan.set(v))
                                            label="气泡内套餐"
                                            width=200
                                        />
                                    </div>
                                </Popover>
                            </div>
                            <span class="c-subtitle">
                                "打开时焦点进面板第一个可聚焦元素，Esc / 点外关闭，关闭后回到触发器。"
                            </span>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"提示"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3" style="align-items:center">
                            <Tooltip label="收藏" placement=TooltipPlacement::Top>
                                <IconButton icon=IconName::STAR label="收藏" />
                            </Tooltip>
                            <Tooltip label="复制链接" placement=TooltipPlacement::Bottom>
                                <IconButton icon=IconName::COPY label="复制链接" />
                            </Tooltip>
                            <Tooltip label="加入书签" placement=TooltipPlacement::Left>
                                <IconButton icon=IconName::BOOKMARK label="加入书签" />
                            </Tooltip>
                            <span class="c-subtitle">"悬停 400ms；聚焦立即出现。"</span>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"滤镜容器里"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--3">
                            // 这个容器带 `transform`，会给 `position: fixed` 造一个包含块。
                            // 浮层如果没有 Portal 到 body，就会按这个盒子的坐标摆 —— 一眼能看出来。
                            // 它同时是回归护栏：以后谁把 Portal 去掉，这一节先崩。
                            <div
                                style="transform:translateZ(0);max-width:560px;border:1px dashed var(--line-strong);border-radius:var(--r-md);padding:var(--s-4);background:var(--surface-sunken)"
                            >
                                <div class="l-row l-row--3" style="align-items:center;flex-wrap:wrap">
                                    <Select
                                        options=vec![
                                            SelectOption::new("free", "免费版"),
                                            SelectOption::new("pro", "专业版"),
                                            SelectOption::new("team", "团队版"),
                                        ]
                                        label="套餐"
                                        width=160
                                    />
                                    <MenuButton label="文件" icon=IconName::FILE_TEXT
                                        items=file_menu() on_select=on_pick />
                                    <Popover open=Signal::derive(move || tf_pop.get())
                                        placement=PopoverPlacement::BottomStart
                                        on_close=Callback::new(move |_| tf_pop.set(false))
                                        label="容器内气泡"
                                        trigger=Box::new(move || view! {
                                            <Button label="容器内气泡"
                                                on_click=Callback::new(move |_| tf_pop.set(true)) />
                                        }.into_any())>
                                        <span class="c-subtitle">"面板挂在 body 上，所以不受这个盒子的坐标影响。"</span>
                                    </Popover>
                                </div>
                            </div>
                            <span class="c-subtitle" style="white-space:normal">
                                "容器带 transform → 它会给 position:fixed 造包含块。浮层一律 Portal 到 body 并按视口坐标定位，所以下拉/菜单/气泡仍贴着各自的触发器。"
                            </span>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"面板与抽屉"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3">
                            <Button label="打开流程面板" variant=ButtonVariant::Primary
                                on_click=Callback::new(move |_| sheet_open.set(true)) />
                            <Button label="打开抽屉"
                                on_click=Callback::new(move |_| drawer_open.set(true)) />
                            <span class="c-subtitle">"Md 套玻璃档；抽屉贴右。"</span>
                        </div>
                    </Card>
                </div>

                <Dialog open=Signal::derive(move || show.get())
                    title="这是一个对话框"
                    description="由 Rust 组件渲染；没有命令式 DOM 操作。"
                    width=380 dismissible=true
                    on_close=Callback::new(move |_| show.set(false))>
                    <p class="c-subtitle" style="margin:12px 0 18px">
                        "Esc 关闭、点遮罩关闭；焦点在打开时进入面板。"
                    </p>
                    <div class="c-dialog__actions">
                        <Button label="取消" variant=ButtonVariant::Plain
                            on_click=Callback::new(move |_| show.set(false)) />
                        <Button label="知道了" variant=ButtonVariant::Primary
                            on_click=Callback::new(move |_| {
                                show.set(false);
                                toast_with("已关闭", Tone::Success, None);
                            }) />
                    </div>
                </Dialog>

                <Sheet open=Signal::derive(move || sheet_open.get())
                    size=SheetSize::Md
                    title="导出为静态站点"
                    description="流程层：标题栏固定、内容可滚动、动作区永远贴底。"
                    on_close=Callback::new(move |_| sheet_open.set(false))
                    footer=Box::new(move || view! {
                        <Button label="取消" variant=ButtonVariant::Plain
                            on_click=Callback::new(move |_| sheet_open.set(false)) />
                        <Button label="开始导出" variant=ButtonVariant::Primary
                            on_click=Callback::new(move |_| {
                                sheet_open.set(false);
                                toast_with("已开始导出", Tone::Success, None);
                            }) />
                    }.into_any())>
                    <p class="c-subtitle">
                        "Md 档 520×360，正好命中玻璃贴图。"
                    </p>
                </Sheet>

                <Drawer open=Signal::derive(move || drawer_open.get())
                    side=DrawerSide::Right width=320
                    on_close=Callback::new(move |_| drawer_open.set(false))>
                    <div class="l-stack l-stack--3">
                        <span class="c-subtitle">"右侧抽屉，宽 320。"</span>
                        <Button label="关闭抽屉" variant=ButtonVariant::Plain
                            on_click=Callback::new(move |_| drawer_open.set(false)) />
                    </div>
                </Drawer>
            </div>
        </div>
    }
}
