use leptos::html;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::base::id::next_id;
use crate::base::overlay::{
    active_element, listen_document, schedule, GlobalListener, OverlayHost,
    OverlayId, OverlayKind,
};

/// 与触发器之间的固定间距。属于「定位几何」，不是视觉令牌：
/// 换成 CSS 变量会让 JS 与 CSS 各算一份，反而更容易漂。
const TRIGGER_GAP: f64 = 6.0;
/// 子菜单与父项的水平间距（旧库是 2px：再大就会和父项脱开视觉联系）。
const SUBMENU_GAP: f64 = 2.0;
/// 视口安全边距：菜单贴边时留一点，避免圆角被窗口切掉。
const VIEWPORT_EDGE: f64 = 8.0;
/// 悬停进入子菜单的延迟。太短会在鼠标横扫整列时一路弹出每一层。
const HOVER_DELAY_MS: i32 = 300;
/// 连打字母的有效窗口：超过这个时间就当成新的前缀。
const TYPEAHEAD_WINDOW_MS: f64 = 800.0;

/// 菜单项。`checked: Some(_)` 时语义是 menuitemradio（CSS 按 `aria-checked` 画点）。
/// `children` 非空时是子菜单父项，点 / → 展开。
#[derive(Clone, PartialEq, Debug, Default)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<IconName>,
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub danger: bool,
    pub checked: Option<bool>,
    pub separator_before: bool,
    pub children: Vec<MenuItem>,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// `checked` 有值即 menuitemradio；传 false 表示「可勾选但当前没选」。
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }

    pub fn separator_before(mut self) -> Self {
        self.separator_before = true;
        self
    }

    pub fn children(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children;
        self
    }
}

/// 一层菜单的共享上下文。全部是 Copy，闭包里可以随便拿。
#[derive(Clone, Copy)]
struct MenuCtx {
    host: OverlayHost,
    /// 收起整串（根菜单关闭）；子菜单激活叶子项也走它。
    close_all: Callback<()>,
    on_select: Option<Callback<String>>,
}

/// 子菜单的句柄：父项只持有它，不需要知道子面板内部怎么排。
#[derive(Clone, Copy)]
struct ChildLink {
    open: RwSignal<bool>,
    pos: RwSignal<(f64, f64)>,
    panel: NodeRef<html::Div>,
    /// 子菜单是从哪个父项打开的：关闭时把焦点还给它。
    parent_el: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    /// 只有键盘打开的子菜单搬过焦点，关闭时才需要还回去。
    focus_moved: StoredValue<bool, LocalStorage>,
}

type LinkStore = StoredValue<Vec<Option<ChildLink>>, LocalStorage>;

fn target_html(ev: &web_sys::Event) -> Option<web_sys::HtmlElement> {
    ev.current_target()?.dyn_into::<web_sys::HtmlElement>().ok()
}

/// 面板的**直接子节点**里找到 `node` 对应的元素。
/// 不用 `query_selector_all`：那会把并列渲染的子菜单项也算进来。
fn direct_child(panel: &web_sys::HtmlElement, node: &web_sys::Node) -> Option<web_sys::HtmlElement> {
    let children = panel.children();
    for i in 0..children.length() {
        let Some(el) = children.item(i) else { continue };
        if el.is_same_node(Some(node)) {
            return el.dyn_into::<web_sys::HtmlElement>().ok();
        }
    }
    None
}

/// 面板的直接子项里「可以落焦点」的那些。
///
/// 必须只看直接子项：子菜单是 DOM 里的兄弟节点，本来就不会混进来；但一旦有人把
/// 子菜单嵌回面板，`query_selector_all` 会把孙项也捞出来，所以在源头就按 children 过滤。
fn enabled_items(panel: &web_sys::HtmlElement) -> Vec<web_sys::HtmlElement> {
    let children = panel.children();
    let mut out = Vec::new();
    for i in 0..children.length() {
        let Some(el) = children.item(i) else { continue };
        if !el.class_list().contains("c-menu__item") {
            continue;
        }
        if el.get_attribute("aria-disabled").as_deref() == Some("true") {
            continue;
        }
        if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
            out.push(el);
        }
    }
    out
}

fn focus_item(el: &web_sys::HtmlElement) {
    let _ = el.focus();
}

/// 打开菜单时焦点落在第一个可用项（不是容器）：
/// 容器虽然带 tabindex="-1"，但焦点停在容器上时读屏不会念出任何一项，
/// 用户还得先按一次 ↓ 才知道菜单里有什么。
fn focus_first_item(panel: &web_sys::HtmlElement) {
    if let Some(el) = enabled_items(panel).first() {
        focus_item(el);
    }
}

fn move_focus(panel: &web_sys::HtmlElement, delta: i32) {
    let list = enabled_items(panel);
    if list.is_empty() {
        return;
    }
    let active = active_element();
    let at = active
        .as_ref()
        .and_then(|a| {
            let a: &web_sys::Node = a.unchecked_ref();
            list.iter()
                .position(|e| e.is_same_node(Some(a)))
                .map(|i| i as i32)
        })
        .unwrap_or(-1);
    let n = list.len() as i32;
    let next = if at < 0 {
        if delta > 0 {
            0
        } else {
            n - 1
        }
    } else {
        (at + delta).rem_euclid(n)
    };
    focus_item(&list[next as usize]);
}

fn focus_edge(panel: &web_sys::HtmlElement, last: bool) {
    let list = enabled_items(panel);
    if let Some(el) = if last { list.last() } else { list.first() } {
        focus_item(el);
    }
}

/// 连打字母定位。缓冲与时间戳都放在面板自己的 StoredValue 里 ——
/// 子菜单各自一份，不会出现「在子菜单里打字却跳到父菜单项」。
fn typeahead(panel: &web_sys::HtmlElement, buf: &StoredValue<(String, f64), LocalStorage>, ch: &str) {
    let now = js_sys::Date::now();
    let (mut acc, last) = buf.get_value();
    if now - last > TYPEAHEAD_WINDOW_MS {
        acc.clear();
    }
    acc.push_str(&ch.to_lowercase());
    buf.set_value((acc.clone(), now));

    let list = enabled_items(panel);
    if list.is_empty() {
        return;
    }
    let active = active_element();
    let start = active
        .as_ref()
        .and_then(|a| {
            let a: &web_sys::Node = a.unchecked_ref();
            list.iter().position(|e| e.is_same_node(Some(a)))
        })
        .unwrap_or(0);
    let n = list.len();
    for step in 0..n {
        let el = &list[(start + step) % n];
        let label = el
            .get_attribute("data-label")
            .unwrap_or_default()
            .to_lowercase();
        if label.starts_with(&acc) {
            focus_item(el);
            return;
        }
    }
}

/// 视口尺寸。`inner_width/inner_height` 在 web-sys 里是 `JsValue`（属性没有类型信息），
/// 统一在这里转成 f64，免得每个定位算式都写一遍 as_f64。
fn viewport() -> (f64, f64) {
    let Some(w) = web_sys::window() else {
        return (1280.0, 800.0);
    };
    let width = w
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let height = w
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    (width, height)
}

/// 打开子菜单：先按父项矩形落位，下一帧量到真实尺寸后再决定是否向左翻。
fn open_sub(link: ChildLink, item: &web_sys::HtmlElement, focus: bool) {
    let r = item.get_bounding_client_rect();
    link.parent_el.set_value(Some(item.clone()));
    link.focus_moved.set_value(focus);
    link.pos.set((r.right() + SUBMENU_GAP, r.top()));
    link.open.set(true);

    let (view_w, view_h) = viewport();
    schedule(move || {
        let Some(panel) = link.panel.get() else { return };
        let w = panel.offset_width() as f64;
        let h = panel.offset_height() as f64;
        let (mut left, mut top) = link.pos.get_untracked();
        // 右边放不下就翻到父项左侧（旧库的「向左」规则）。
        if left + w > view_w - VIEWPORT_EDGE {
            left = r.left() - w - SUBMENU_GAP;
        }
        left = left.max(VIEWPORT_EDGE);
        let max_top = (view_h - h - VIEWPORT_EDGE).max(VIEWPORT_EDGE);
        top = top.clamp(VIEWPORT_EDGE, max_top);
        link.pos.set((left, top));
        if focus {
            focus_first_item(&panel);
        }
    });
}

/// 同一层里只允许一个子菜单：鼠标横扫时先收起别的，再开当前的。
fn close_other_subs(links: LinkStore, keep: Option<usize>) {
    for (i, link) in links.get_value().iter().enumerate() {
        if let Some(link) = link {
            if Some(i) != keep && link.open.get_untracked() {
                link.open.set(false);
            }
        }
    }
}

fn activate(ctx: MenuCtx, item: &web_sys::HtmlElement) {
    if item.get_attribute("aria-disabled").as_deref() == Some("true") {
        return;
    }
    if let Some(id) = item.get_attribute("data-id") {
        if let Some(cb) = ctx.on_select {
            cb.run(id);
        }
    }
    ctx.close_all.run(());
}

/// 建一层菜单（根菜单或子菜单）。
///
/// 子菜单**不嵌在**父面板里：`.c-menu` 带 backdrop-filter 与 overflow:hidden，
/// 它既是 fixed 的包含块又会裁切，嵌进去的子菜单一定被切掉。所以每层都是并列的
/// `.c-menu`，用各自的 OverlayId 记 z —— 深度天然由栈给出。
///
/// 宽度一律交给内容（CSS `min-width:180px`）或玻璃档：按触发器宽度写死内联
/// `width` 会把「文件」这种短标签的菜单压到 100px，快捷键整列被裁掉。
#[allow(clippy::too_many_arguments)]
fn build_panel(
    items: Vec<MenuItem>,
    level: usize,
    ctx: MenuCtx,
    visible: Signal<bool>,
    set_visible: Callback<bool>,
    pos: RwSignal<(f64, f64)>,
    owner: String,
    panel_ref: NodeRef<html::Div>,
    glass: Option<&'static str>,
    label: String,
) -> AnyView {
    let layer = RwSignal::new(None::<OverlayId>);
    let host = ctx.host;
    let hover_timer = StoredValue::new_local(None::<i32>);
    let typeahead_buf = StoredValue::new_local((String::new(), 0.0f64));

    // 先给每个带子项的条目建句柄，再渲染条目 —— 条目的事件闭包要用它。
    let links: Vec<Option<ChildLink>> = items
        .iter()
        .map(|item| {
            if item.children.is_empty() {
                None
            } else {
                Some(ChildLink {
                    open: RwSignal::new(false),
                    pos: RwSignal::new((0.0, 0.0)),
                    panel: NodeRef::new(),
                    parent_el: StoredValue::new_local(None),
                    focus_moved: StoredValue::new_local(false),
                })
            }
        })
        .collect();
    let links_store: LinkStore = StoredValue::new_local(links);

    let mut entries: Vec<AnyView> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let link = links_store.get_value()[index];
        let has_children = !item.children.is_empty();
        let checked = item.checked;
        // 有子项的条目优先级更高：它必须是 menuitem + aria-haspopup，
        // 不能同时挂 aria-checked（radio + 子菜单在 ARIA 里是非法组合）。
        let has_check = checked.is_some() && !has_children;
        let role = if has_check { "menuitemradio" } else { "menuitem" };
        let mut cls = String::from("c-menu__item");
        if item.danger {
            cls.push_str(" c-menu__item--danger");
        }
        let item_id = item.id.clone();
        let item_label = item.label.clone();
        let shortcut = item.shortcut.clone();
        let icon = item.icon;
        let disabled = item.disabled;
        let is_checked = checked.unwrap_or(false);

        let on_enter = move |ev: web_sys::PointerEvent| {
            if disabled {
                return;
            }
            let Some(el) = target_html(ev.unchecked_ref()) else { return };
            close_other_subs(links_store, Some(index));
            let Some(link) = link else { return };
            if let Some(id) = hover_timer.get_value() {
                if let Some(w) = web_sys::window() {
                    w.clear_timeout_with_handle(id);
                }
            }
            let target = el.clone();
            let link_open = link.open;
            let cb = Closure::once_into_js(move || {
                if !link_open.get_untracked() {
                    open_sub(link, &target, false);
                }
            });
            if let Some(w) = web_sys::window() {
                if let Ok(id) = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.unchecked_ref::<js_sys::Function>(),
                    HOVER_DELAY_MS,
                ) {
                    hover_timer.set_value(Some(id));
                }
            }
        };

        let on_leave = move |_: web_sys::PointerEvent| {
            if let Some(id) = hover_timer.get_value() {
                if let Some(w) = web_sys::window() {
                    w.clear_timeout_with_handle(id);
                }
                hover_timer.set_value(None);
            }
        };

        let on_click = move |ev: web_sys::MouseEvent| {
            if disabled {
                return;
            }
            let Some(el) = target_html(ev.unchecked_ref()) else { return };
            match link {
                // 父项点击 = 展开/收起下一层（不是激活），所以不关整串。
                Some(link) => {
                    if link.open.get_untracked() {
                        link.open.set(false);
                    } else {
                        open_sub(link, &el, true);
                    }
                }
                None => activate(ctx, &el),
            }
        };

        let mut row: Vec<AnyView> = Vec::new();
        if has_check {
            row.push(
                view! {
                    <span class="c-menu__check" data-role="menuitemradio">
                        {is_checked.then(|| view! { <Icon name=IconName::CHECK size=12 /> })}
                    </span>
                }
                .into_any(),
            );
        }
        if let Some(name) = icon {
            row.push(view! { <Icon name=name size=14 /> }.into_any());
        }
        row.push(view! { <span class="c-menu__label">{item_label.clone()}</span> }.into_any());
        if let Some(keys) = shortcut {
            // CSS 里这个类叫 .c-menu__key（不是 __shortcut），按实际类名写。
            row.push(view! { <span class="c-menu__key">{keys}</span> }.into_any());
        }
        if has_children {
            row.push(
                view! {
                    <span class="c-menu__sub-arrow">
                        <Icon name=IconName::CHEVRON_RIGHT size=11 />
                    </span>
                }
                .into_any(),
            );
        }
        let row = row.into_iter().collect_view();

        let item_view = view! {
            <div
                class=cls
                role=role
                tabindex="-1"
                data-id=item_id
                data-label=item_label
                data-index=index.to_string()
                aria-checked=has_check.then(|| is_checked.to_string())
                aria-disabled=disabled.then_some("true")
                aria-haspopup=has_children.then_some("menu")
                aria-expanded=move || has_children.then(|| {
                    link.map(|l| l.open.get().to_string()).unwrap_or_else(|| "false".into())
                })
                on:pointerenter=on_enter
                on:pointerleave=on_leave
                on:click=on_click
            >
                {row}
            </div>
        };
        if item.separator_before {
            entries.push(view! { <div class="c-menu__sep" role="separator"></div> }.into_any());
        }
        entries.push(item_view.into_any());
    }
    let body = entries.into_iter().collect_view();

    let panel_style = move || {
        if !visible.get() {
            return "display:none".to_string();
        }
        let (left, top) = pos.get();
        let z = layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Menu.base_z());
        format!("left:{left}px;top:{top}px;z-index:{z}")
    };

    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.is_composing() || ev.key_code() == 229 {
            return;
        }
        let Some(panel) = panel_ref.get() else { return };
        // 当前聚焦项：先定位它在本面板的直接子项里，再按 data-index 找回原始条目。
        let record = active_element().and_then(|a| {
            let node: &web_sys::Node = a.unchecked_ref();
            direct_child(&panel, node)
        });
        let record_link = record.as_ref().and_then(|el| {
            el.get_attribute("data-index")
                .and_then(|v| v.parse::<usize>().ok())
                .and_then(|i| links_store.get_value().get(i).copied().flatten())
        });

        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                close_other_subs(links_store, None);
                move_focus(&panel, 1);
            }
            "ArrowUp" => {
                ev.prevent_default();
                close_other_subs(links_store, None);
                move_focus(&panel, -1);
            }
            "Home" => {
                ev.prevent_default();
                focus_edge(&panel, false);
            }
            "End" => {
                ev.prevent_default();
                focus_edge(&panel, true);
            }
            "ArrowRight" => {
                let (Some(el), Some(link)) = (record.as_ref(), record_link) else {
                    return;
                };
                if el.get_attribute("aria-haspopup").as_deref() == Some("menu") {
                    ev.prevent_default();
                    open_sub(link, el, true);
                }
            }
            "ArrowLeft" => {
                // 只退一层：这一层收掉，父项由它自己的关闭逻辑接住焦点。
                if level == 0 {
                    return;
                }
                ev.prevent_default();
                set_visible.run(false);
            }
            "Enter" | " " | "Spacebar" => {
                let Some(el) = record.as_ref() else { return };
                ev.prevent_default();
                match record_link {
                    Some(link) if el.get_attribute("aria-haspopup").as_deref() == Some("menu") => {
                        open_sub(link, el, true)
                    }
                    _ => activate(ctx, el),
                }
            }
            "Tab" => {
                // 菜单不是模态：Tab 收起整串，焦点由 Menu 的关闭逻辑送回触发器。
                // preventDefault 是为了避免浏览器在「当前项正被 display:none」的
                // 瞬间把焦点丢给 body。
                ev.prevent_default();
                ctx.close_all.run(());
            }
            key if key.len() == 1 && !ev.meta_key() && !ev.ctrl_key() && !ev.alt_key() => {
                typeahead(&panel, &typeahead_buf, key);
            }
            _ => {}
        }
    };

    // 入栈/出栈跟着本层可见性走。Esc 由 OverlayHost 派发到栈顶 ——
    // 子菜单是后入栈的，所以第一次 Esc 只收子菜单。
    Effect::new(move |_| {
        if visible.get() {
            if layer.get_untracked().is_none() {
                let id = host.push(
                    OverlayKind::Menu,
                    Callback::new(move |_| set_visible.run(false)),
                );
                layer.set(Some(id));
            }
        } else if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
    });

    on_cleanup(move || {
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    let panel = view! {
        <div
            class="c-menu"
            role="menu"
            aria-label=label
            tabindex="-1"
            data-menu-owner=owner.clone()
            data-glass=glass
            node_ref=panel_ref
            style=panel_style
            on:keydown=on_key
        >
            {body}
        </div>
    }
    .into_any();

    // 子菜单面板并列渲染：父面板 + 每一层的子面板（子面板自己再递归）。
    let mut parts: Vec<AnyView> = vec![panel];
    for (index, item) in items.iter().enumerate() {
        let Some(link) = links_store.get_value()[index] else {
            continue;
        };
        let parent = visible;
        let child_visible = Signal::derive(move || parent.get() && link.open.get());
        let child = build_panel(
            item.children.clone(),
            level + 1,
            ctx,
            child_visible,
            Callback::new(move |v: bool| link.open.set(v)),
            link.pos,
            owner.clone(),
            link.panel,
            None,
            item.label.clone(),
        );
        // 子菜单关闭时把焦点还给父项：挂在子面板上，父项元素由 link 带过去。
        let restored = link;
        Effect::new(move |_| {
            if !child_visible.get() && parent.get_untracked() {
                if restored.focus_moved.get_value() {
                    restored.focus_moved.set_value(false);
                    if let Some(el) = restored.parent_el.get_value() {
                        if el.is_connected() {
                            let _ = el.focus();
                        }
                    }
                }
            }
        });
        parts.push(child);
    }
    parts.into_iter().collect_view().into_any()
}

/// 纯面板菜单：`open` + `anchor(left, top, width)` 由调用方控制，组件只负责
/// 定位、键盘与栈。`fixed_size` 为真时套 `menu-240x280` 玻璃档（高 280，内部滚动）。
#[component]
pub fn Menu(
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(into, optional)] anchor: MaybeProp<(f64, f64, f64)>,
    #[prop(into)] items: Vec<MenuItem>,
    #[prop(into, optional)] label: String,
    #[prop(optional)] fixed_size: bool,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let host = OverlayHost::use_or_provide();
    let root_ref = NodeRef::<html::Div>::new();
    let pos = RwSignal::new((0.0f64, 0.0f64));
    let owner = next_id("gd-menu");
    let owner_key = StoredValue::new(owner.clone());
    let return_to = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
    // 关闭时是否归还焦点。点外关闭不归还（用户已经去别处了），Esc / Tab / 选中都归还。
    let restore = StoredValue::new_local(true);
    let label = if label.is_empty() {
        "菜单".to_string()
    } else {
        label
    };

    let is_open = move || open.get().unwrap_or(false);
    let close = move || {
        if let Some(cb) = on_close {
            cb.run(());
        }
    };
    let close_all = Callback::new(move |_| close());
    let ctx = MenuCtx {
        host,
        close_all,
        on_select,
    };

    let glass = fixed_size.then_some("menu-240x280");

    let panel = build_panel(
        items,
        0,
        ctx,
        Signal::derive(move || is_open()),
        Callback::new(move |v: bool| {
            if !v {
                close();
            }
        }),
        pos,
        owner.clone(),
        root_ref,
        glass,
        label,
    );

    Effect::new(move |_| {
        if is_open() {
            let (left, top, _) = anchor.get().unwrap_or((0.0, 0.0, 0.0));
            pos.set((left, top));
            // 打开前的焦点就是「触发器」：菜单不是模态，归还只能自己做。
            // 只在第一次入栈时记录 —— effect 若因 anchor 变化重跑，焦点已经在菜单里了。
            if return_to.get_value().is_none() {
                return_to.set_value(active_element());
            }
            restore.set_value(true);
            // 点外关闭：文档级 pointerdown 比全屏遮罩稳 —— 遮罩会盖住触发器，
            // 用户点触发器想收起时会先关再开，看起来像「点了个寂寞」。
            if outside.with_value(|v| v.is_none()) {
                let owner = owner_key.get_value();
                let sel = format!("[data-menu-owner='{owner}']");
                let close = close_all;
                let restore = restore;
                if let Some(listener) = listen_document("pointerdown", move |ev| {
                    let inside = ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .and_then(|el| el.closest(&sel).ok().flatten())
                        .is_some();
                    if !inside {
                        restore.set_value(false);
                        close.run(());
                    }
                }) {
                    outside.set_value(Some(listener));
                }
            }
            // 下一帧量真实尺寸：锚点只给了「下边缘」，溢出时按高度翻到上方，
            // 水平越界则夹进视口（右侧菜单在窄窗口里会贴右）。
            schedule(move || {
                let Some(panel) = root_ref.get() else { return };
                let w = panel.offset_width() as f64;
                let h = panel.offset_height() as f64;
                let (view_w, view_h) = viewport();
                let (left, top, _) = anchor.get_untracked().unwrap_or((0.0, 0.0, 0.0));
                let mut next_left = left;
                let mut next_top = top;
                if next_left + w > view_w - VIEWPORT_EDGE {
                    next_left = view_w - w - VIEWPORT_EDGE;
                }
                next_left = next_left.max(VIEWPORT_EDGE);
                if next_top + h > view_h - VIEWPORT_EDGE {
                    next_top = top - h - TRIGGER_GAP;
                }
                if next_top < VIEWPORT_EDGE {
                    next_top = VIEWPORT_EDGE;
                }
                pos.set((next_left, next_top));
                focus_first_item(&panel);
            });
        } else {
            outside.set_value(None);
            // 归还焦点：只有「这次关闭该还」且焦点确实落在菜单里才动。
            // 用标志位而不是查 activeElement —— 此刻面板可能已经被 display:none，
            // 浏览器已经把焦点丢回 body，查 DOM 永远得到「不在里面」。
            if restore.get_value() {
                if let Some(el) = return_to.get_value() {
                    schedule(move || {
                        if el.is_connected() {
                            let _ = el.focus();
                        }
                    });
                }
            }
            restore.set_value(true);
            return_to.set_value(None);
        }
    });

    panel
}

/// 触发器 + 菜单。触发器自己算锚点，所以调用方只要给条目。
/// `fixed_size` 打开 `data-glass="menu-240x280"`：菜单高度锁 280，项多了内部滚动。
#[component]
pub fn MenuButton(
    #[prop(into, optional)] label: String,
    #[prop(optional)] icon: Option<IconName>,
    #[prop(into)] items: Vec<MenuItem>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] fixed_size: bool,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let anchor = RwSignal::new((0.0f64, 0.0f64, 0.0f64));
    let trigger_ref = NodeRef::<html::Button>::new();
    // 记录「pointerdown 时菜单已经开着」：Menu 的点外监听会先把菜单关掉，
    // 紧接着的 click 若照常 toggle 就会立刻又打开 —— 看上去像没反应。
    let pressed_open = StoredValue::new_local(false);
    let items_for_menu = items.clone();

    let mut cls = String::from("c-btn");
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let open_menu = move || {
        if let Some(el) = trigger_ref.get() {
            // 先确保焦点在触发器上：菜单关闭时要靠 activeElement 归还焦点，
            // 而程序化打开（或某些指针设备）不一定会自动聚焦按钮。
            let _ = el.focus();
            let r = el.get_bounding_client_rect();
            anchor.set((r.left(), r.bottom() + TRIGGER_GAP, r.width()));
        }
        open.set(true);
    };

    view! {
        <button
            class=cls
            type="button"
            aria-haspopup="menu"
            aria-expanded=move || open.get().to_string()
            node_ref=trigger_ref
            on:pointerdown=move |_| pressed_open.set_value(open.get_untracked())
            on:click=move |_| {
                if pressed_open.get_value() {
                    pressed_open.set_value(false);
                    return;
                }
                if open.get_untracked() {
                    open.set(false);
                } else {
                    open_menu();
                }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "ArrowDown" || ev.key() == "ArrowUp" {
                    ev.prevent_default();
                    open_menu();
                }
            }
        >
            {icon.map(|name| view! { <Icon name=name size=14 /> })}
            <span class="c-btn__label">{label.clone()}</span>
            <span
                style="display:grid"
                style:transform=move || format!("rotate({}deg)", if open.get() { 180 } else { 0 })
            >
                <Icon name=IconName::CHEVRON_DOWN size=12 />
            </span>
        </button>
        <Menu
            open=Signal::derive(move || open.get())
            anchor=anchor
            items=items_for_menu
            label=label.clone()
            fixed_size=fixed_size
            // 组件之间转发 Option<Callback> 时要先落地成具体回调：prop 的
            // `#[prop(optional)]` 只接受 T，不接受 Option<T>。
            on_select=on_select.unwrap_or_else(|| Callback::new(|_| {}))
            on_close=Callback::new(move |_| open.set(false))
        />
    }
}
