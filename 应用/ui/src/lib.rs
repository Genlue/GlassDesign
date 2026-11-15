use glass_design::prelude::*;
use leptos::prelude::*;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    glass_design::install();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let theme = Theme::provide();

    let checked = RwSignal::new(true);
    let name = RwSignal::new("未命名".to_string());
    let saved = RwSignal::new(false);
    let section = RwSignal::new("基础");

    let toggle_theme = Callback::new(move |_| {
        theme.set_mode(if theme.mode.get() == ThemeMode::Dark {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        });
    });

    view! {
        <AppShell>
            <Toolbar>
                <ToolbarLead>
                    <ToolbarTitle title="GlassDesign" subtitle="Rust 控件库 · P1" />
                </ToolbarLead>
                <ToolbarPills>
                    <Capsule size=CapsuleSize::Round>
                        <IconButton
                            icon=IconName::MOON
                            label="切换明暗"
                            on_click=toggle_theme
                        />
                    </Capsule>
                </ToolbarPills>
            </Toolbar>

            <NavColumn>
                <NavGroup>
                    <NavSection label="控件" />
                    <NavItem
                        icon=IconName::LAYOUT_GRID
                        label="基础"
                        selected=Signal::derive(move || section.get() == "基础")
                        on_click=Callback::new(move |_| section.set("基础"))
                    />
                    <NavItem
                        icon=IconName::SLIDERS
                        label="表单"
                        selected=Signal::derive(move || section.get() == "表单")
                        on_click=Callback::new(move |_| section.set("表单"))
                    />
                    <NavItem
                        icon=IconName::SETTINGS
                        label="设置"
                        selected=Signal::derive(move || section.get() == "设置")
                        on_click=Callback::new(move |_| section.set("设置"))
                    />
                </NavGroup>
            </NavColumn>

            <Main>
                <ScrollArea>
                    <div class="l-scroll" style="padding:24px 32px 64px">
                        <div class="l-stack l-stack--6">
                            <div class="l-stack l-stack--2">
                                <h1 class="c-headline">"GlassDesign"</h1>
                                <p class="c-subtitle">
                                    "Tauri 2 + Leptos 的声明式控件库。这一页由 Rust 组件拼出来。"
                                </p>
                            </div>

                            <div class="l-stack l-stack--3">
                                <SectionTitle>"材质"</SectionTitle>
                                <Card>
                                    <div style="position:relative;height:190px;border-radius:18px;overflow:hidden;background:linear-gradient(115deg,#ff9a62 0%,#ff5f8f 34%,#7b61ff 68%,#2ec5ce 100%)">
                                        <div style="position:absolute;inset:0;background-image:repeating-linear-gradient(90deg,rgba(255,255,255,.20) 0 2px,transparent 2px 28px)"></div>
                                        <div style="position:absolute;left:26px;top:24px;color:#fff;font-size:12px;letter-spacing:.04em;opacity:.9">"静态 SVG filter · 无 JavaScript"</div>
                                        <div style="position:absolute;left:26px;bottom:26px;display:flex;align-items:center;gap:14px">
                                            <Capsule size=CapsuleSize::Md>
                                                <span style="font-size:13px;font-weight:500;color:#fff">"Liquid Glass"</span>
                                            </Capsule>
                                            <span style="color:#fff;font-size:12px;opacity:.78">"位移 + 高光贴图在构建期算好"</span>
                                        </div>
                                    </div>
                                </Card>
                            </div>

                            <div class="l-stack l-stack--3">
                                <SectionTitle>"按钮"</SectionTitle>
                                <Card>
                                    <div class="l-pad l-stack l-stack--4">
                                        <Row label="变体" hint="主操作只放一个">
                                            <div class="l-row l-row--3">
                                                <Button label="默认" />
                                                <Button label="主要" variant=ButtonVariant::Primary />
                                                <Button label="柔和" variant=ButtonVariant::Tonal />
                                                <Button label="危险" variant=ButtonVariant::Danger />
                                            </div>
                                        </Row>
                                        <Row label="带图标">
                                            <div class="l-row l-row--3">
                                                <Button
                                                    label="新建"
                                                    icon=IconName::FOLDER_PLUS
                                                    variant=ButtonVariant::Primary
                                                    on_click=Callback::new(move |_| saved.set(false))
                                                />
                                                <IconButton icon=IconName::REFRESH_CW label="刷新" />
                                                <IconButton
                                                    icon=IconName::TRASH
                                                    label="删除"
                                                    variant=IconButtonVariant::Tonal
                                                />
                                            </div>
                                        </Row>
                                        <Row label="禁用">
                                            <Button label="不可用" disabled=true />
                                        </Row>
                                    </div>
                                </Card>
                            </div>

                            <div class="l-stack l-stack--3">
                                <SectionTitle>"输入"</SectionTitle>
                                <Card>
                                    <div class="l-pad l-stack l-stack--4">
                                        <Row label="名称" hint="受控输入，值在 signal 里">
                                            <Field
                                                value=Signal::derive(move || name.get())
                                                placeholder="输入名称"
                                                icon=IconName::PERSON
                                                on_input=Callback::new(move |v| name.set(v))
                                            />
                                        </Row>
                                        <Row label="启用" hint="点击只上报，视觉由 checked 决定">
                                            <Switch
                                                checked=Signal::derive(move || checked.get())
                                                on_change=Callback::new(move |v| checked.set(v))
                                            />
                                        </Row>
                                    </div>
                                </Card>
                            </div>

                            <Card>
                                <div class="l-pad l-stack l-stack--3">
                                    <SectionTitle>"当前状态"</SectionTitle>
                                    <Row label="名称">
                                        <span class="c-subtitle">{move || name.get()}</span>
                                    </Row>
                                    <Row label="启用">
                                        <span class="c-subtitle">
                                            {move || if checked.get() { "开" } else { "关" }}
                                        </span>
                                    </Row>
                                    <Row label="操作">
                                        <Button
                                            label="保存"
                                            variant=ButtonVariant::Primary
                                            on_click=Callback::new(move |_| saved.set(true))
                                        />
                                    </Row>
                                    <Show when=move || saved.get()>
                                        <div class="c-inline c-inline--success">
                                            <div class="c-inline__body">
                                                <div class="c-inline__title">"已保存"</div>
                                                <div class="c-inline__desc">"这是 P1 的静态反馈，不是真实持久化。"</div>
                                            </div>
                                        </div>
                                    </Show>
                                </div>
                            </Card>
                        </div>
                    </div>
                </ScrollArea>
                <div class="c-topveil"></div>
            </Main>
        </AppShell>
    }
}
