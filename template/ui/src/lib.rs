// 页面入口。
//
// - `leptos::prelude::*`          → `view!` / `#[component]` / `RwSignal` / `IntoView`
// - `glass_design::prelude::*`    → 组件、`bootstrap`、`Theme`、`IconName`
// - `wasm_bindgen`                → wasm 入口宏（直接来自 crate 根）
//
// `#[component]` / `view!` 展开成 `::leptos::…`，`#[wasm_bindgen]` 展开成
// `::wasm_bindgen::…` 绝对路径，所以这两个 crate 必须在 Cargo.toml 里直接声明。
// `glass-design` 是组件来源，也要声明 —— 三个依赖缺一不可。

use glass_design::prelude::*;
use leptos::prelude::*;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    // 一行做完：注入样式与贴图、建主题与浮层栈、挂载根组件。
    bootstrap(App);
}

#[component]
fn App() -> impl IntoView {
    let enabled = RwSignal::new(true);

    view! {
        <AppShell>
            <NavColumn>
                <NavGroup>
                    <NavSection label="主菜单" />
                    <NavItem icon=IconName::HOUSE label="首页" selected=true />
                    <NavItem icon=IconName::SETTINGS label="设置" />
                </NavGroup>
            </NavColumn>
            <Main>
                <ScrollArea>
                    <Pad>
                        <Stack>
                            <SectionTitle>"首页"</SectionTitle>
                            <Card>
                                <Row label="启用玻璃">
                                    <Switch
                                        checked=enabled
                                        on_change=Callback::new(move |v| enabled.set(v))
                                    />
                                </Row>
                                <Row label="状态">
                                    <Chip label="运行中" />
                                </Row>
                            </Card>
                        </Stack>
                    </Pad>
                </ScrollArea>
            </Main>
        </AppShell>
    }
}
