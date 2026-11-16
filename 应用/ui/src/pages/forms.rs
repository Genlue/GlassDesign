use glass_design::prelude::*;
use leptos::prelude::*;

#[component]
pub fn FormPage() -> impl IntoView {
    let name = RwSignal::new("未命名".to_string());
    let search = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let enabled = RwSignal::new(true);
    let agree = RwSignal::new(false);
    let radio = RwSignal::new("day".to_string());
    let density = RwSignal::new("regular".to_string());
    let volume = RwSignal::new(48.0);
    let count = RwSignal::new(3.0);
    let qty = RwSignal::new(12.0);
    let plan = RwSignal::new("pro".to_string());
    let show_dialog = RwSignal::new(false);

    // —— P3-B1：区间 / 组合框 / 令牌 / 日期时间 / 表单编排 ——
    let range_price = RwSignal::new((20.0f64, 80.0f64));
    let range_budget = RwSignal::new((250.0f64, 750.0f64));
    let city = RwSignal::new("hangzhou".to_string());
    let custom_tag = RwSignal::new(String::new());
    let tags = RwSignal::new(vec![
        "设计".to_string(),
        "前端".to_string(),
        "材质".to_string(),
    ]);
    let due = RwSignal::new("2026-11-20".to_string());
    let at24 = RwSignal::new("09:30".to_string());
    let at12 = RwSignal::new("14:05".to_string());
    let due_span = RwSignal::new(("2026-11-14".to_string(), "2026-11-21".to_string()));
    let night = RwSignal::new(("22:00".to_string(), "06:00".to_string()));
    let at_sec = RwSignal::new("18:45:30".to_string());
    let form_name = RwSignal::new(String::new());
    let form_mail = RwSignal::new(String::new());
    let form_note = RwSignal::new(String::new());
    let form_error = RwSignal::new(String::new());

    let cities = vec![
        ComboboxOption::new("beijing", "北京").hint("BJ"),
        ComboboxOption::new("shanghai", "上海").hint("SH"),
        ComboboxOption::new("hangzhou", "杭州").hint("HZ"),
        ComboboxOption::new("shenzhen", "深圳").hint("SZ"),
        ComboboxOption::new("chengdu", "成都").hint("CD"),
        ComboboxOption::new("wuhan", "武汉").hint("WH"),
    ];
    let tag_options = vec![
        ComboboxOption::new("design", "设计"),
        ComboboxOption::new("frontend", "前端"),
        ComboboxOption::new("material", "材质"),
        ComboboxOption::new("motion", "动效"),
    ];

    let name_error = Signal::derive(move || {
        if name.get().trim().is_empty() {
            "名称不能为空".to_string()
        } else {
            String::new()
        }
    });

    let radios = vec![
        RadioOption::new("day", "按日"),
        RadioOption::new("week", "按周"),
        RadioOption::new("month", "按月"),
    ];
    let segments = vec![
        SegmentedOption::new("compact", "紧凑").icon(IconName::LIST),
        SegmentedOption::new("regular", "常规").icon(IconName::GRID_2X2),
    ];
    let plans = vec![
        SelectOption::new("free", "免费版"),
        SelectOption::new("pro", "专业版"),
        SelectOption::new("team", "团队版"),
    ];

    view! {
        <div class="l-scroll" style="padding:24px 32px 64px">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"表单"</h1>
                    <p class="c-subtitle" style="white-space:normal">"受控值、校验态、选择与分段；每个控件的唯一真相都在调用方。"</p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"输入"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Row label="名称" hint="失焦或输入时校验">
                                <FormField required=true error=name_error>
                                    <Field value=Signal::derive(move || name.get())
                                        placeholder="输入名称"
                                        icon=IconName::PERSON
                                        on_input=Callback::new(move |v| name.set(v)) />
                                </FormField>
                            </Row>
                            <Row label="搜索">
                                <SearchField value=Signal::derive(move || search.get())
                                    on_input=Callback::new(move |v| search.set(v)) />
                            </Row>
                            <Row label="备注" hint="Cmd/Ctrl + Enter 提交">
                                <TextArea value=Signal::derive(move || note.get())
                                    placeholder="写点什么…"
                                    on_input=Callback::new(move |v| note.set(v))
                                    on_enter=Callback::new(move |_| { toast("已提交备注"); }) />
                            </Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"选择"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Row label="开关" hint="点击只上报下一个值">
                                <Switch checked=Signal::derive(move || enabled.get())
                                    on_change=Callback::new(move |v| enabled.set(v)) />
                            </Row>
                            <Row label="复选">
                                <Checkbox checked=Signal::derive(move || agree.get())
                                    label="我已阅读并同意"
                                    on_change=Callback::new(move |v| agree.set(v)) />
                            </Row>
                            <Row label="单选">
                                <RadioGroup options=radios value=Signal::derive(move || radio.get())
                                    on_change=Callback::new(move |v| radio.set(v)) />
                            </Row>
                            <Row label="分段">
                                <Segmented options=segments value=Signal::derive(move || density.get())
                                    on_change=Callback::new(move |v| density.set(v)) />
                            </Row>
                            <Row label="下拉">
                                <Select options=plans value=Signal::derive(move || plan.get())
                                    on_change=Callback::new(move |v| plan.set(v)) />
                            </Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"数值"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--5">
                            <Row label="滑块" hint="拖动或方向键">
                                <div style="width:320px">
                                    <Slider value=Signal::derive(move || volume.get())
                                        on_input=Callback::new(move |v| volume.set(v))
                                        on_change=Callback::new(move |v| volume.set(v)) />
                                </div>
                            </Row>
                            <Row label="步进">
                                <div class="l-row l-row--3" style="align-items:center">
                                    <Stepper value=Signal::derive(move || count.get()) min=0.0 max=10.0 step=1.0
                                        on_change=Callback::new(move |v| count.set(v)) />
                                    <span class="c-subtitle">{move || format!("当前：{}", count.get())}</span>
                                </div>
                            </Row>
                            <Row label="数值输入" hint="失焦或回车提交">
                                <div style="width:140px">
                                    <NumberField value=Signal::derive(move || qty.get()) min=0.0 max=99.0 step=1.0
                                        label="数量"
                                        on_change=Callback::new(move |v| qty.set(v)) />
                                </div>
                            </Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"反馈"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--3">
                            <Button label="打开对话框" variant=ButtonVariant::Primary
                                on_click=Callback::new(move |_| show_dialog.set(true)) />
                            <Button label="提示条" icon=IconName::INFO
                                on_click=Callback::new(move |_| { toast("已保存"); }) />
                            <Button label="成功提示" icon=IconName::CIRCLE_CHECK
                                on_click=Callback::new(move |_| { toast_with("操作成功", Tone::Success, None); }) />
                            <Button label="危险提示" icon=IconName::X_CIRCLE variant=ButtonVariant::Danger
                                on_click=Callback::new(move |_| { toast_with("操作失败", Tone::Danger, None); }) />
                        </div>
                    </Card>
                </div>

                <Dialog open=Signal::derive(move || show_dialog.get())
                    title="确认操作" description="这个对话框由 Rust 组件渲染，Esc 或点遮罩关闭。"
                    width=360 dismissible=true
                    on_close=Callback::new(move |_| show_dialog.set(false))>
                    <p class="c-subtitle" style="margin:12px 0 18px">"所有状态仍在 signal 里，没有命令式 DOM 操作。"</p>
                    <div class="c-dialog__actions">
                        <Button label="取消" variant=ButtonVariant::Plain
                            on_click=Callback::new(move |_| show_dialog.set(false)) />
                        <Button label="确定" variant=ButtonVariant::Primary
                            on_click=Callback::new(move |_| {
                                show_dialog.set(false);
                                toast_with("已确认", Tone::Success, None);
                            }) />
                    </div>
                </Dialog>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"区间滑块"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--6">
                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"价格区间 · 0–100 / 步长 1 · 谁近谁动"</span>
                                <div class="l-row l-row--3" style="align-items:center;max-width:420px">
                                    <div style="flex:1;min-width:260px">
                                        <RangeSlider
                                            value=Signal::derive(move || range_price.get())
                                            min=0.0 max=100.0 step=1.0 label="价格"
                                            on_input=Callback::new(move |v| range_price.set(v))
                                            on_change=Callback::new(move |v| range_price.set(v)) />
                                    </div>
                                    <span class="c-rangeslider__value">
                                        {move || {
                                            let (lo, hi) = range_price.get();
                                            format!("{lo} – {hi}")
                                        }}
                                    </span>
                                </div>
                            </div>
                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"预算区间 · 0–1000 / 步长 50"</span>
                                <div class="l-row l-row--3" style="align-items:center;max-width:420px">
                                    <div style="flex:1;min-width:260px">
                                        <RangeSlider
                                            value=Signal::derive(move || range_budget.get())
                                            min=0.0 max=1000.0 step=50.0 label="预算"
                                            on_input=Callback::new(move |v| range_budget.set(v))
                                            on_change=Callback::new(move |v| range_budget.set(v)) />
                                    </div>
                                    <span class="c-rangeslider__value">
                                        {move || {
                                            let (lo, hi) = range_budget.get();
                                            format!("{lo} – {hi}")
                                        }}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"组合框"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--6">
                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"城市 · ↓↑ 移动、Enter 选中、Esc 收起"</span>
                                <div style="width:320px">
                                    <Combobox options=cities
                                        value=Signal::derive(move || city.get())
                                        label="城市" placeholder="搜索城市"
                                        on_change=Callback::new(move |v| city.set(v)) />
                                </div>
                            </div>
                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"标签 · 允许自定义，输入后回车即提交"</span>
                                <div style="width:320px">
                                    <Combobox options=tag_options
                                        value=Signal::derive(move || custom_tag.get())
                                        label="标签" placeholder="输入或选择标签" allow_custom=true
                                        on_change=Callback::new(move |v| custom_tag.set(v)) />
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"令牌"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <span class="c-subtitle">"回车 / 逗号添加；空输入框 Backspace 删最后一枚；上限 6"</span>
                            <div style="width:380px">
                                <TokenField tokens=Signal::derive(move || tags.get())
                                    label="标签" placeholder="输入标签后回车" max=6
                                    on_change=Callback::new(move |v| tags.set(v)) />
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"日期与时间"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--6">
                            <Row label="日期" hint="可选范围 2026-11-01 ~ 2026-12-31">
                                <DatePicker value=Signal::derive(move || due.get())
                                    min="2026-11-01" max="2026-12-31"
                                    label="截止日期"
                                    on_change=Callback::new(move |v| due.set(v)) />
                            </Row>
                            <Row label="时间 24h" hint="↑↓ 步进，Shift ×5，Enter 提交">
                                <TimePicker value=Signal::derive(move || at24.get())
                                    label="开会时间"
                                    on_change=Callback::new(move |v| at24.set(v)) />
                            </Row>
                            <Row label="时间 12h" hint="三段：时 / 分 / 上午下午">
                                <TimePicker value=Signal::derive(move || at12.get())
                                    label="提醒时间" hour12=true
                                    on_change=Callback::new(move |v| at12.set(v)) />
                            </Row>
                            <Row label="日期区间" hint="两次点击定起止；先点止再点起会自动交换">
                                <DateRange value=Signal::derive(move || due_span.get())
                                    min="2026-11-01" max="2026-12-31"
                                    label="账期"
                                    on_change=Callback::new(move |v| due_span.set(v)) />
                            </Row>
                            <Row label="时间区间" hint="跨零点合法：22:00 → 06:00 是八小时">
                                <TimeRange
                                    start=Signal::derive(move || night.get().0)
                                    end=Signal::derive(move || night.get().1)
                                    label="值守时段"
                                    on_change=Callback::new(move |v| night.set(v)) />
                            </Row>
                            <Row label="带秒" hint="值写成 HH:mm:ss，秒按 15 秒步进">
                                <TimePicker value=Signal::derive(move || at_sec.get())
                                    label="采集时刻" show_seconds=true step_secs=15
                                    on_change=Callback::new(move |v| at_sec.set(v)) />
                            </Row>
                            <Row label="网格" hint="三列：↑↓ 格内移动、←→ 换列、PageUp/Down 翻一屏">
                                <TimePicker value=Signal::derive(move || at_sec.get())
                                    label="网格时间" grid=true show_seconds=true step_secs=15
                                    on_change=Callback::new(move |v| at_sec.set(v)) />
                            </Row>
                            <span class="c-subtitle">
                                {move || {
                                    format!(
                                        "{} · {} · {} · {} → {} · {} → {} · {}",
                                        due.get(),
                                        at24.get(),
                                        at12.get(),
                                        due_span.get().0,
                                        due_span.get().1,
                                        night.get().0,
                                        night.get().1,
                                        at_sec.get(),
                                    )
                                }}
                            </span>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"表单编排"</SectionTitle>
                    <Card>
                        <div class="l-pad" style="max-width:420px">
                            <Form
                                error=Signal::derive(move || form_error.get())
                                on_submit=Callback::new(move |_| {
                                    if form_name.get().trim().is_empty() {
                                        form_error.set("名称不能为空".to_string());
                                        return;
                                    }
                                    form_error.set(String::new());
                                    toast("已提交");
                                })
                                on_cancel=Callback::new(move |_| {
                                    form_name.set(String::new());
                                    form_mail.set(String::new());
                                    form_note.set(String::new());
                                    form_error.set(String::new());
                                })
                            >
                                <FormField label="名称" required=true>
                                    <Field value=Signal::derive(move || form_name.get())
                                        placeholder="必填"
                                        on_input=Callback::new(move |v| form_name.set(v)) />
                                </FormField>
                                <FormField label="邮箱" hint="不会公开">
                                    <Field value=Signal::derive(move || form_mail.get())
                                        placeholder="name@example.com"
                                        on_input=Callback::new(move |v| form_mail.set(v)) />
                                </FormField>
                                <FormField label="备注">
                                    <TextArea value=Signal::derive(move || form_note.get())
                                        placeholder="补充说明…"
                                        on_input=Callback::new(move |v| form_note.set(v)) />
                                </FormField>
                                <FormActions submit_label="提交" cancel_label="重置" />
                            </Form>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}
