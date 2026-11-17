//! 「声明式拼装」的最小样本：整页没有一行手写 CSS，也没有一次 DOM 操作。
//!
//! 结构就三层：外框 `AppShell`、顶栏 `Toolbar`、左栏 `NavColumn`、内容 `Main`。
//! 玻璃、圆角、间距都写在组件自带的类名里 —— 调用方只决定「谁挨着谁」。
//!
//! 状态是普通的 `RwSignal`：`Switch` 受控，点击只上报下一个值，
//! 视觉永远由你手里那个信号决定；`toast()` 是全局提示，不用层层传 props。
//!
//! 入口只有一行 `bootstrap(App)`，样式注入、主题、语言、浮层栈都在里面。
//! 要改默认主题或语言，换成 `bootstrap_with`（见 crate 文档）。
//!
//! 宿主目标上 `main` 是空的：这个例子要 wasm32 + 浏览器才有意义，
//! 但 `cargo check --examples` 得能在开发机上过 —— 那也是门面的一部分。

use glass_design::prelude::*;
use leptos::prelude::*;

// 非 wasm 目标不调 `bootstrap`，`App` 就没人引用；不是错误，别让它刷警告。
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[component]
fn App() -> impl IntoView {
    // 0 = 偏好，1 = 关于。导航项的 selected 直接读它，没有第二份状态。
    let page = RwSignal::new(0u32);
    let reduce_motion = RwSignal::new(true);
    let dense = RwSignal::new(false);

    view! {
        <AppShell>
            <Toolbar>
                // 顶栏第一项必须是窗口区：标题组的位移账本按它的宽度算。
                <ToolbarChrome>
                    <TrafficLights
                        focused=Signal::derive(|| true)
                        // 浏览器预览没有真窗口：给一条可见反馈，别做成点了没反应。
                        on_close=Callback::new(|_| {
                            toast("这是预览，没有窗口可关");
                        })
                    />
                </ToolbarChrome>
                <ToolbarLead>
                    <ToolbarTitle title="玻璃控制台" subtitle="GlassDesign · 声明式拼装" />
                </ToolbarLead>
            </Toolbar>

            <NavColumn>
                <NavGroup>
                    <NavSection label="示例" />
                    <NavItem
                        icon=IconName::HOUSE
                        label="偏好"
                        selected=Signal::derive(move || page.get() == 0)
                        on_click=Callback::new(move |_| page.set(0))
                    />
                    <NavItem
                        icon=IconName::SETTINGS
                        label="关于"
                        selected=Signal::derive(move || page.get() == 1)
                        on_click=Callback::new(move |_| page.set(1))
                    />
                </NavGroup>
            </NavColumn>

            <Main>
                <ScrollArea>
                    <div class="gd-page">
                        <div class="l-stack l-stack--6">
                            {move || {
                                if page.get() == 0 {
                                    settings(reduce_motion, dense).into_any()
                                } else {
                                    about().into_any()
                                }
                            }}
                        </div>
                    </div>
                </ScrollArea>
            </Main>
        </AppShell>
    }
}

fn settings(reduce_motion: RwSignal<bool>, dense: RwSignal<bool>) -> impl IntoView {
    view! {
        <div class="l-stack l-stack--2">
            <h1 class="c-headline">"偏好设置"</h1>
            <p class="c-subtitle" style="white-space:normal">
                "两个受控开关：点击只上报下一个值，视觉由 RwSignal 决定。"
            </p>
            <div class="l-row l-row--3">
                <Badge label="v0.8.0" variant=BadgeVariant::Accent />
                <Badge label="Rust" />
                <Badge label="CSR" />
            </div>
        </div>

        <div class="l-stack l-stack--3">
            <SectionTitle>"开关"</SectionTitle>
            <Card>
                <div class="l-pad l-stack l-stack--4">
                    <Row label="减少动效" hint="关掉玻璃上的位移动画">
                        <Switch
                            checked=Signal::derive(move || reduce_motion.get())
                            label="减少动效"
                            on_change=Callback::new(move |v| reduce_motion.set(v))
                        />
                    </Row>
                    <Row label="紧凑模式" hint="收窄行距与内边距">
                        <Switch
                            checked=Signal::derive(move || dense.get())
                            label="紧凑模式"
                            on_change=Callback::new(move |v| dense.set(v))
                        />
                    </Row>
                </div>
            </Card>
        </div>

        <div class="l-stack l-stack--3">
            <SectionTitle>"动作"</SectionTitle>
            <Card>
                <div class="l-pad l-stack l-stack--4">
                    <Row label="按钮" hint="主操作只放一个；危险动作用红色">
                        <div class="l-row l-row--3">
                            <Button
                                label="保存"
                                icon=IconName::CIRCLE_CHECK
                                variant=ButtonVariant::Primary
                                on_click=Callback::new(|_| {
                                    toast("已保存（演示而已）");
                                })
                            />
                            <Button
                                label="重置"
                                icon=IconName::REFRESH_CW
                                variant=ButtonVariant::Danger
                                on_click=Callback::new(move |_| {
                                    reduce_motion.set(true);
                                    dense.set(false);
                                    toast("已恢复默认");
                                })
                            />
                        </div>
                    </Row>
                </div>
            </Card>
        </div>
    }
}

fn about() -> impl IntoView {
    view! {
        <div class="l-stack l-stack--2">
            <h1 class="c-headline">"关于这个例子"</h1>
            <p class="c-subtitle" style="white-space:normal">
                "整页只有一棵 view! 树和几个 RwSignal，没有手写样式表，也没有命令式操作。"
            </p>
        </div>

        <div class="l-stack l-stack--3">
            <SectionTitle>"入口"</SectionTitle>
            <Card>
                <div class="l-pad l-stack l-stack--4">
                    <Row label="一行装完" hint="样式 · 主题 · 语言 · 浮层栈">
                        <span class="c-subtitle" style="white-space:normal">
                            "glass_design::bootstrap(App);"
                        </span>
                    </Row>
                    <Row label="样式自己引" hint="wasm 少 465 KB，首帧就有样式">
                        <span class="c-subtitle" style="white-space:normal">
                            "bootstrap_with(App, BootstrapOptions { install: false, ..Default::default() })"
                        </span>
                    </Row>
                </div>
            </Card>
        </div>
    }
}

fn main() {
    // 桌面上跑起来才有意义；宿主目标留空，`cargo check --examples` 才过。
    #[cfg(target_arch = "wasm32")]
    glass_design::bootstrap(App);
}
