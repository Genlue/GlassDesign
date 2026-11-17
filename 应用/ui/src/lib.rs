mod pages;

use glass_design::prelude::*;
use leptos::prelude::*;
use pages::*;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    glass_design::install();
    leptos::mount::mount_to_body(App);
}

/// 右键菜单的接管。
///
/// 三层判断，缺一层就会变成「在输入框里右键弹的是应用菜单，没法粘贴」：
/// 1. 输入类元素（input / textarea / contenteditable）**放行**给系统菜单 ——
///    剪贴板是系统能力，自己接管反而把「粘贴」弄丢；
/// 2. 其余位置 `preventDefault`，浏览器菜单不再出现；
/// 3. 打开前把焦点交给 body。`Menu` 的坐标 API 会优先拿「打开瞬间的焦点元素」
///    当锚点，不清掉就会贴着上一次点过的按钮弹，而不是跟着鼠标。
fn install_context_menu(ctx: RwSignal<Option<(f64, f64)>>) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let doc_for_cb = doc.clone();
    let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let editable = target
                .closest("input, textarea, [contenteditable='true']")
                .ok()
                .flatten()
                .is_some();
            if editable {
                return;
            }
        }
        ev.prevent_default();
        if let Some(active) = doc_for_cb.active_element() {
            if let Some(html) = active.dyn_ref::<web_sys::HtmlElement>() {
                let _ = html.blur();
            }
        }
        ctx.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
    });
    let _ = doc.add_event_listener_with_callback("contextmenu", cb.as_ref().unchecked_ref());
    // 根组件不卸载，监听器要活到进程结束 —— 这里就是它的归属。
    std::mem::forget(cb);
}

#[component]
fn App() -> impl IntoView {
    let theme = Theme::provide();
    // 界面语言：控件库内部只认 `Locale`，切换器与 provider 都在这一层，
    // 所以工具栏那个分段与所有控件的文案看的是同一个信号。
    let locale = RwSignal::new_local(Locale::ZhCn);
    provide_locale(locale);
    let section = RwSignal::new("基础".to_string());

    // 导航形态：side = 侧栏占位；dock = 侧栏收走、导航浮成药丸。
    // 与旧库一致 —— 两种形态共用同一份页面清单，不是两套 UI。
    let nav_form = RwSignal::new("side".to_string());
    let nav_hidden = RwSignal::new(false);
    let dock_edge = RwSignal::new("left".to_string());

    // 主题三态。旧库是 sun / moon / monitor 三选一，不是「一个按钮来回切」——
    // 少了「跟随系统」这一档，用户就再也没法回到自动。
    let theme_options = vec![
        SegmentedOption::new("light", "").icon(IconName::SUN).title("浅色"),
        SegmentedOption::new("dark", "").icon(IconName::MOON).title("深色"),
        SegmentedOption::new("system", "").icon(IconName::MONITOR).title("跟随系统"),
    ];
    let theme_value = Signal::derive(move || theme.mode.get().as_str().to_string());
    let set_theme = Callback::new(move |v: String| {
        theme.set_mode(match v.as_str() {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
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

    let nav_form_options = vec![
        SegmentedOption::new("side", "").icon(IconName::PANEL_LEFT).title("侧栏"),
        SegmentedOption::new("dock", "").icon(IconName::SQUARE_DASHED).title("悬浮导航"),
    ];
    let nav_form_value = Signal::derive(move || nav_form.get());
    let set_nav_form = Callback::new(move |v: String| {
        nav_form.set(v);
        // 从悬浮切回侧栏时，别把「收起」一起带回来。
        nav_hidden.set(false);
    });

    let edge_options = vec![
        SegmentedOption::new("left", "左").title("靠左"),
        SegmentedOption::new("right", "右").title("靠右"),
        SegmentedOption::new("bottom", "底").title("靠底"),
    ];
    let edge_value = Signal::derive(move || dock_edge.get());
    let set_edge = Callback::new(move |v: String| dock_edge.set(v));

    // 右侧波浪条：当前页的分区目录。切页后重新扫描 DOM 填进去。
    let wave_items = RwSignal::new(Vec::<WaveItem>::new());
    Effect::new(move |_| {
        let _ = section.get();
        scan_sections(wave_items);
    });

    let page = move || match section.get().as_str() {
        "表单" => view! { <FormPage /> }.into_any(),
        "外观" => view! { <AppearancePage /> }.into_any(),
        "数据" => view! { <DataPage /> }.into_any(),
        "导航" => view! { <NavPage /> }.into_any(),
        "面板" => view! { <PanelsPage /> }.into_any(),
        "工具栏" => view! { <ToolbarPage /> }.into_any(),
        "浮层" => view! { <OverlayPage /> }.into_any(),
        "调节" => view! { <TunerPage /> }.into_any(),
        _ => view! { <BasicPage /> }.into_any(),
    };

    // —— 右键菜单 ——
    let ctx = RwSignal::new(None::<(f64, f64)>);
    install_context_menu(ctx);
    let ctx_items = vec![
        MenuItem::new("new", "新建文件").icon(IconName::FOLDER_PLUS).shortcut("⌘N"),
        MenuItem::new("open", "打开…").icon(IconName::FOLDER_OPEN),
        MenuItem::new("dup", "复制一份").icon(IconName::COPY).separator_before(),
        MenuItem::new("inspect", "在检查器中显示")
            .icon(IconName::PANEL_RIGHT)
            .separator_before(),
        MenuItem::new("del", "移到废纸篓").icon(IconName::TRASH).danger(),
    ];

    view! {
        // 悬浮形态下侧栏必须收走：两种形态共用同一份导航数据，
        // 同时出现会变成两套导航互相打脸。
        <AppShell nav_hidden=Signal::derive(move || nav_hidden.get() || nav_form.get() == "dock")>
            <Toolbar>
                // 顶栏第一项必须是窗口区：标题组的位移账本按它算宽度。
                <ToolbarChrome>
                    <TrafficLights
                        focused=Signal::derive(move || true)
                        on_close=Callback::new(move |_| {
                            // 画廊里没有真窗口可关：给一条可见反馈，别做成「点了没反应」。
                            toast("这是浏览器预览，没有窗口可关");
                        })
                    />
                    // 收起/展开：必须挂在 `.l-chrome__toggle` 上，CSS 靠它把按钮
                    // 顶到导航栏右上角；换了类名就落回顶栏左侧。
                    {move || {
                        let hidden = nav_hidden.get();
                        let dock = nav_form.get() == "dock";
                        let (icon, label) = match (hidden, dock) {
                            (false, false) => (IconName::PANEL_LEFT, "收起导航栏"),
                            (true, false) => (IconName::PANEL_RIGHT, "展开导航栏"),
                            (false, true) => (IconName::PANEL_LEFT, "收起悬浮导航"),
                            (true, true) => (IconName::PANEL_RIGHT, "展开悬浮导航"),
                        };
                        view! {
                            // 用 Auto + `--circle`，**不要用 Round**：
                            // Round 会挂 `url(#gd-glass-capsule-34)` 这张构建期贴图，
                            // 贴图里的镜面高光是烘死的 —— 把「高光」调到 0 时，
                            // 其它胶囊都干净了，只有它还留着半圈描边。
                            // 另外 Round 自带 width:34 与 padding:0 4px 打架，
                            // 内件 34 撑出 42，贴图还按 34 画，看起来就是重影。
                            <Capsule class="l-chrome__toggle c-capsule--circle">
                                <IconButton
                                    icon=icon
                                    label=label
                                    on_click=Callback::new(move |_| nav_hidden.update(|h| *h = !*h))
                                />
                            </Capsule>
                        }
                    }}
                </ToolbarChrome>
                <ToolbarLead>
                    <ToolbarTitle title="GlassDesign" subtitle=format!("Rust 控件库 · v{VERSION}") />
                </ToolbarLead>
                <ToolbarPills>
                    <Capsule>
                        <Segmented
                            options=nav_form_options
                            value=nav_form_value
                            on_change=set_nav_form
                        />
                    </Capsule>
                    // 停靠边只对悬浮形态有意义：侧栏没有「靠哪一边」这回事。
                    {move || {
                        if nav_form.get() != "dock" {
                            return None;
                        }
                        // 每次现 clone：闭包要能反复调用，不能把 Vec move 进去。
                        let opts = edge_options.clone();
                        Some(view! {
                            <Capsule>
                                <Segmented options=opts value=edge_value on_change=set_edge />
                            </Capsule>
                        })
                    }}
                    <Capsule>
                        <Segmented
                            options=theme_options.clone()
                            value=theme_value
                            on_change=set_theme
                        />
                    </Capsule>
                    // 分段控件整枚 69px 宽。**不能再套固定档位**：
                    // Sm = 112×34 是给「尺寸已知、要吃到 112 折射贴图」的胶囊用的，
                    // 把 69px 的内容塞进 112px 的外壳，两侧各留 21px 空气，
                    // 读起来就是「一个被拉长的圈」而不是「包住内容的胶囊」。
                    // Auto 不给 data-size → 不套贴图，宽度跟着内件走（外壳内件永远差 3px）。
                    <Capsule>
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
                    <NavSection label="开发" />
                    <NavItem icon=IconName::SPARKLES label="调节"
                        selected=Signal::derive(move || section.get() == "调节")
                        on_click=Callback::new(move |_| section.set("调节".to_string())) />
                </NavGroup>
            </NavColumn>

            // 悬浮导航：和侧栏互斥，由上面的形态分段切换。
            {move || (nav_form.get() == "dock").then(|| view! {
                <NavDock
                    items=dock_items()
                    value=Signal::derive(move || section.get())
                    on_select=Callback::new(move |id: String| section.set(id))
                    orientation=match dock_edge.get().as_str() {
                        "right" => DockOrientation::Right,
                        "bottom" => DockOrientation::Bottom,
                        _ => DockOrientation::Left,
                    }
                    label="页面"
                    hidden=Signal::derive(move || nav_hidden.get())
                />
            })}

            <Main>
                <ScrollArea>{page}</ScrollArea>
                <div class="c-topveil"></div>
                // 页内目录：每根横条是一个分区，当前那根最长，邻近按高斯衰减。
                // 点击滚过去；滚动时它自己认当前分区（`sec-*` 的 id 由 scan 写）。
                <SectionWave items=Signal::derive(move || wave_items.get()) />
            </Main>

            // 右键菜单：坐标由 install_context_menu 写进 ctx，
            // 面板本身走 Menu（Portal + 栈 + Esc + 翻转夹取），不另起一套。
            <Menu
                open=Signal::derive(move || ctx.get().is_some())
                anchor=Signal::derive(move || {
                    let (x, y) = ctx.get().unwrap_or((0.0, 0.0));
                    (x, y, 180.0)
                })
                items=ctx_items
                label="上下文菜单"
                // 右键菜单不预选第一项：还没指过去就有一项亮着，不是"选中"是噪音。
                focus_first=false
                on_select=Callback::new(move |id: String| {
                    toast(format!("右键菜单：{id}"));
                    ctx.set(None);
                })
                on_close=Callback::new(move |_| ctx.set(None))
            />
        </AppShell>
    }
}

/// 悬浮导航的条目 —— 和左栏同一份页面清单，不另建数据源。
fn dock_items() -> Vec<DockItem> {
    [
        ("基础", IconName::LAYOUT_GRID),
        ("表单", IconName::SLIDERS),
        ("外观", IconName::PALETTE),
        ("数据", IconName::GRID_2X2),
        ("导航", IconName::COMPASS),
        ("面板", IconName::PANEL_RIGHT),
        ("工具栏", IconName::SQUARE_DASHED),
        ("浮层", IconName::LAYERS),
        ("调节", IconName::SPARKLES),
    ]
    .into_iter()
    .map(|(name, icon)| DockItem::new(name, name, icon))
    .collect()
}

/// 扫当前页的分区标题，给每个分区块写 `sec-<i>`，并换成波浪条的条目。
///
/// 为什么扫 DOM 而不是让每个页面声明一份目录：页面里已经有 `SectionTitle`，
/// 再手写一份清单就等于同一件事写两遍，改标题时必然漏。这里把「标题即目录」
/// 作为唯一事实，页面文件一行都不用改。
///
/// 必须等 DOM 挂完：切页是把整棵子树换掉，同步读会读到上一页。
/// rAF 落在补丁之后、绘制之前，是能拿到新节点又不闪的最早时机。
fn scan_sections(out: RwSignal<Vec<WaveItem>>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb = Closure::once_into_js(move || {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(nodes) = doc.query_selector_all(".gd-page .c-section") else {
            return;
        };
        let mut items = Vec::new();
        for i in 0..nodes.length() {
            let Some(node) = nodes.get(i) else { continue };
            let Ok(el) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let label = el.text_content().unwrap_or_default().trim().to_string();
            if label.is_empty() {
                continue;
            }
            // 标题本身就是分区块的开头（`.c-section` 是 `SectionTitle` 的类名），
            // 滚到它即可；再往外找 `.l-stack` 会把上一张卡也算进这一节。
            let host = el.clone();
            // WaveItem.id 是**裸 key**：组件自己拼 `sec-` 前缀去找元素
            // （旧库的约定：section id = `sec-<key>`）。这里把前缀也算进来，
            // 它就会去找 `sec-sec-0`，点击静默失效。
            let _ = host.set_id(&format!("sec-{i}"));
            items.push(WaveItem { id: i.to_string(), label });
        }
        out.set(items);
    });
    let _ = window.request_animation_frame(cb.unchecked_ref::<js_sys::Function>());
}
