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
    // 界面语言：控件库内部只认 `Locale`，切换器与 provider 都在这一层，
    // 所以工具栏那个分段与所有控件的文案看的是同一个信号。
    let locale = RwSignal::new_local(Locale::ZhCn);
    provide_locale(locale);
    let section = RwSignal::new("基础".to_string());

    let toggle_theme = Callback::new(move |_| {
        theme.set_mode(if theme.mode.get() == ThemeMode::Dark {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        });
    });

    let lang_options = vec![
        SegmentedOption::new("zh", "中").title("中文"),
        SegmentedOption::new("en", "EN").title("English"),
    ];
    let lang_value = Signal::derive(move || match locale.get() {
        Locale::ZhCn => "zh".to_string(),
        Locale::En => "en".to_string(),
    });
    let set_lang = Callback::new(move |v: String| {
        locale.set(if v == "en" { Locale::En } else { Locale::ZhCn });
    });

    let page = move || match section.get().as_str() {
        "表单" => view! { <FormPage /> }.into_any(),
        "外观" => view! { <AppearancePage /> }.into_any(),
        "数据" => view! { <DataPage /> }.into_any(),
        "导航" => view! { <NavPage /> }.into_any(),
        "面板" => view! { <PanelsPage /> }.into_any(),
        "工具栏" => view! { <ToolbarPage /> }.into_any(),
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
                    <Capsule size=CapsuleSize::Round>
                        <Segmented
                            options=lang_options
                            value=lang_value
                            on_change=set_lang
                        />
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
                    <NavItem icon=IconName::PALETTE label="外观"
                        selected=Signal::derive(move || section.get() == "外观")
                        on_click=Callback::new(move |_| section.set("外观".to_string())) />
                    <NavItem icon=IconName::GRID_2X2 label="数据"
                        selected=Signal::derive(move || section.get() == "数据")
                        on_click=Callback::new(move |_| section.set("数据".to_string())) />
                    <NavItem icon=IconName::COMPASS label="导航"
                        selected=Signal::derive(move || section.get() == "导航")
                        on_click=Callback::new(move |_| section.set("导航".to_string())) />
                    <NavItem icon=IconName::PANEL_RIGHT label="面板"
                        selected=Signal::derive(move || section.get() == "面板")
                        on_click=Callback::new(move |_| section.set("面板".to_string())) />
                    <NavItem icon=IconName::SQUARE_DASHED label="工具栏"
                        selected=Signal::derive(move || section.get() == "工具栏")
                        on_click=Callback::new(move |_| section.set("工具栏".to_string())) />
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
