mod pages;

use glass_design::prelude::*;
use leptos::prelude::*;
use pages::*;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    glass_design::install();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let theme = Theme::provide();
    let section = RwSignal::new("基础".to_string());

    let toggle_theme = Callback::new(move |_| {
        theme.set_mode(if theme.mode.get() == ThemeMode::Dark {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        });
    });

    let page = move || match section.get().as_str() {
        "表单" => view! { <FormPage /> }.into_any(),
        "数据" => view! { <DataPage /> }.into_any(),
        "导航" => view! { <NavPage /> }.into_any(),
        "浮层" => view! { <OverlayPage /> }.into_any(),
        _ => view! { <BasicPage /> }.into_any(),
    };

    view! {
        <AppShell>
            <Toolbar>
                <ToolbarLead>
                    <ToolbarTitle title="GlassDesign" subtitle=format!("Rust 控件库 · v{VERSION}") />
                </ToolbarLead>
                <ToolbarPills>
                    <Capsule size=CapsuleSize::Round>
                        <IconButton icon=IconName::MOON label="切换明暗" on_click=toggle_theme />
                    </Capsule>
                </ToolbarPills>
            </Toolbar>

            <NavColumn>
                <NavGroup>
                    <NavSection label="控件" />
                    <NavItem icon=IconName::LAYOUT_GRID label="基础"
                        selected=Signal::derive(move || section.get() == "基础")
                        on_click=Callback::new(move |_| section.set("基础".to_string())) />
                    <NavItem icon=IconName::SLIDERS label="表单"
                        selected=Signal::derive(move || section.get() == "表单")
                        on_click=Callback::new(move |_| section.set("表单".to_string())) />
                    <NavItem icon=IconName::GRID_2X2 label="数据"
                        selected=Signal::derive(move || section.get() == "数据")
                        on_click=Callback::new(move |_| section.set("数据".to_string())) />
                    <NavItem icon=IconName::COMPASS label="导航"
                        selected=Signal::derive(move || section.get() == "导航")
                        on_click=Callback::new(move |_| section.set("导航".to_string())) />
                    <NavItem icon=IconName::LAYERS label="浮层"
                        selected=Signal::derive(move || section.get() == "浮层")
                        on_click=Callback::new(move |_| section.set("浮层".to_string())) />
                </NavGroup>
            </NavColumn>

            <Main>
                <ScrollArea>{page}</ScrollArea>
                <div class="c-topveil"></div>
            </Main>
        </AppShell>
    }
}
