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
                    <p class="c-subtitle">"受控值、校验态、选择与分段；每个控件的唯一真相都在调用方。"</p>
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
            </div>
        </div>
    }
}
