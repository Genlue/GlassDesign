use glass_design::prelude::*;
use leptos::prelude::*;

#[component]
pub fn OverlayPage() -> impl IntoView {
    let show = RwSignal::new(false);
    let plan = RwSignal::new("pro".to_string());
    let options = vec![
        SelectOption::new("free", "免费版"),
        SelectOption::new("pro", "专业版"),
        SelectOption::new("team", "团队版"),
    ];

    view! {
        <div class="l-scroll" style="padding:24px 32px 64px">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"浮层"</h1>
                    <p class="c-subtitle">"对话框、下拉与提示条。Esc / 遮罩 / 点外关闭。"</p>
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
            </div>
        </div>
    }
}
