use std::cell::Cell;
use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::anchor::{self, AnchorSpec, AnchorWatch, Placement, Side};
use crate::base::i18n::t;
use crate::base::icon::{Icon, IconName};
use crate::base::id::next_id;
use crate::base::overlay::{
    active_element, focus_is_free, listen_document, listen_window, schedule, ExitGate,
    GlobalListener, OverlayHost, OverlayId, OverlayKind,
};

/// 与触发器之间的固定间距。属于「定位几何」，不是视觉令牌：
/// 换成 CSS 变量会让 JS 与 CSS 各算一份，反而更容易漂。
const TRIGGER_GAP: f64 = 6.0;
/// 子菜单与父项的水平间距。比根菜单的 6 略小：子菜单要和父项保持「连着」的视觉关系。
const SUBMENU_GAP: f64 = 4.0;
/// 菜单最小宽度，与 CSS `.c-menu { min-width: 180px }` 同一档。
/// 宽度仍然按内容算（按触发器宽度写死会把「文件」这种短标签压到 100px，快捷键整列被裁）。
const MENU_MIN_WIDTH: f64 = 180.0;
/// 菜单最大高度：超过就在面板内部滚动。
const MENU_MAX_HEIGHT: f64 = 420.0;
/// 悬停进入子菜单的延迟。太短会在鼠标横扫整列时一路弹出每一层。
const HOVER_DELAY_MS: i32 = 300;
/// 连打字母的有效窗口：超过这个时间就当成新的前缀。
const TYPEAHEAD_WINDOW_MS: f64 = 800.0;

/// 根菜单的定位规格：贴触发器下方、按内容宽、越界翻上与夹取全交给共享层。
fn root_spec() -> AnchorSpec {
    AnchorSpec {
        gap: TRIGGER_GAP,
        match_width: false,
        min_width: MENU_MIN_WIDTH,
        max_height: MENU_MAX_HEIGHT,
        ..AnchorSpec::dropdown()
    }
}

/// 子菜单的定位规格：不锁宽、不按可用高度截断，与父项缝 4。
///
/// `scroll` 单独改成 `true`：`bubble()` 是给不滚动的气泡用的，但子菜单项多时
/// 必须能滚 —— 只靠 CSS 的 `max-height:100vh-16px`，否则把 `overflow-y` 写死成
/// visible 后长菜单会溢出视口、底下的项点不到。
fn sub_spec() -> AnchorSpec {
    AnchorSpec {
        gap: SUBMENU_GAP,
        scroll: true,
        ..AnchorSpec::bubble()
    }
}

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
    panel: NodeRef<html::Div>,
    /// 子菜单是从哪个父项打开的：既是锚点，也是关闭时焦点的去处。
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
/// 必须只看直接子项：子菜单是 body 下的兄弟节点，本来就不会混进来；但一旦有人把
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

/// 打开子菜单：只记「锚在哪个父项上」。
/// 落位必须等 DOM 落定（要量高度），所以放在子面板自己的 Effect 里，不在这里算。
fn open_sub(link: ChildLink, item: &web_sys::HtmlElement, focus: bool) {
    link.parent_el.set_value(Some(item.clone()));
    link.focus_moved.set_value(focus);
    link.open.set(true);
}

/// 子菜单落位：贴父项**右侧**、与父项顶对齐；右侧放不下就翻到左侧，再夹进视口。
///
/// 共享层只做「下方/上方翻转 + 与锚点左对齐」，左右翻转是子菜单独有的规则，
/// 所以先用 `anchor::place` 拿到宽度与内容高度，再在这里补横向。
/// 第二项是横向落点，写进 `data-sub-side`（`data-side` 已被共享层写成 below/above）。
fn place_sub(
    anchor_el: &web_sys::HtmlElement,
    panel: &web_sys::HtmlElement,
    spec: AnchorSpec,
) -> (Placement, &'static str) {
    let base = anchor::place(anchor_el, panel, spec);
    let a = anchor_el.get_bounding_client_rect();
    let (vw, vh) = viewport();
    let width = base.width;

    let mut left = a.right() + spec.gap;
    let mut dir = "right";
    if left + width > vw - spec.margin {
        // 右侧放不下：翻到父项左边（旧库的「向左」规则）。
        left = a.left() - width - spec.gap;
        dir = "left";
    }
    if left < spec.margin {
        // 两侧都放不下（窗口比两层菜单还窄）：夹进视口，宁可压住父面板也不出界。
        left = (vw - width - spec.margin).max(spec.margin);
        dir = "left";
    }

    // `bubble()` 的 max-height 是 INFINITY，这里只借它算「顶对齐 + 夹进视口」。
    let avail_h = (vh - spec.margin * 2.0).max(0.0);
    let height = (panel.offset_height() as f64).min(avail_h);
    let top = a
        .top()
        .clamp(spec.margin, (vh - height - spec.margin).max(spec.margin));
    let side = if top >= a.top() {
        Side::Below
    } else {
        Side::Above
    };

    (
        Placement {
            side,
            left,
            top,
            width,
            max_height: spec.max_height,
        },
        dir,
    )
}

/// 子菜单的跟随句柄：滚动 / resize 后按父项重新落位。
///
/// 不复用 `anchor::watch`：它只会「贴下方 + 左对齐」，直接用它每滚动一次就会把
/// 子菜单掰回父项左下方。跟随这一层必须知道左右翻转规则，所以自己挂两个监听。
struct SubFollow {
    _scroll: Option<GlobalListener>,
    _resize: Option<GlobalListener>,
}

fn follow_sub(
    anchor_el: web_sys::HtmlElement,
    panel: web_sys::HtmlElement,
    on_gone: Rc<dyn Fn()>,
) -> SubFollow {
    // rAF 合帧：滚动一次会来几十个事件，每次都量布局会掉帧。
    let queued = Rc::new(Cell::new(false));
    let pump: Rc<dyn Fn()> = {
        let anchor_el = anchor_el.clone();
        let panel = panel.clone();
        let queued = Rc::clone(&queued);
        Rc::new(move || {
            if queued.replace(true) {
                return;
            }
            let anchor_el = anchor_el.clone();
            let panel = panel.clone();
            let queued = Rc::clone(&queued);
            let on_gone = Rc::clone(&on_gone);
            schedule(move || {
                queued.set(false);
                if !anchor_el.is_connected() || !panel.is_connected() {
                    return;
                }
                // 父项被滚出视口：留一个挂在屏幕外的子菜单没有意义。
                let (_, vh) = viewport();
                let a = anchor_el.get_bounding_client_rect();
                if a.bottom() <= 0.0 || a.top() >= vh {
                    on_gone();
                    return;
                }
                let spec = sub_spec();
                let (placed, dir) = place_sub(&anchor_el, &panel, spec);
                anchor::apply(&panel, placed, spec);
                let _ = panel.set_attribute("data-sub-side", dir);
            });
        })
    };

    // scroll 不冒泡，必须 capture=true 才能收到内部滚动容器的事件。
    SubFollow {
        _scroll: listen_window("scroll", true, {
            let pump = Rc::clone(&pump);
            move |_| pump()
        }),
        _resize: listen_window("resize", false, move |_| pump()),
    }
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
/// 每层都是**独立**的 `.c-menu`：既各自 `<Portal>` 到 body，也各自带一个 OverlayId 记 z
/// （深度天然由栈给出，Esc 也天然逐层）。不能嵌进父面板：`.c-menu` 带 backdrop-filter 与
/// overflow:hidden，它既是 fixed 的包含块又会裁切，嵌进去的子菜单一定被切掉。
///
/// `parent_layer` 是**上一层**的 OverlayId：子菜单靠它声明 lineage，浮层栈才不会把
/// 父菜单当成「无关浮层」一起收掉。根菜单传 `None`。
///
/// 宽度一律交给内容（CSS `min-width:180px` 或玻璃档）。
#[allow(clippy::too_many_arguments)]
fn build_panel(
    items: Vec<MenuItem>,
    level: usize,
    ctx: MenuCtx,
    visible: Signal<bool>,
    set_visible: Callback<bool>,
    owner: String,
    panel_ref: NodeRef<html::Div>,
    glass: Option<&'static str>,
    label: String,
    parent_layer: Option<RwSignal<Option<OverlayId>>>,
) -> AnyView {
    let layer = RwSignal::new(None::<OverlayId>);
    let host = ctx.host;
    // 退场态：`visible` 已经 false，但面板还留在屏幕上播过渡、也还留在栈上。
    // 每层各自一份 —— 子菜单和父菜单可以同时退场，各自等自己的票。
    let closing = RwSignal::new(false);
    let gate = ExitGate::new();
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

    // 第一帧的样式：先藏到视口外，等 `anchor::apply` 量完再显形。
    // `width:max-content` 是为了让 `offset_width` 就是「最宽那一行」；玻璃档
    // （menu-240x280）的宽高由 CSS 写死，不能被它顶掉，所以只在非玻璃档写。
    //
    // 「在场上」判的是栈句柄而不是 `visible`：关闭后要先留在屏幕上播完退场动画，
    // 出栈（`layer` 归 None）才是真正的收起时刻。
    let panel_style = move || {
        if layer.get().is_none() {
            return "display:none".to_string();
        }
        let z = layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Menu.base_z());
        let width = if glass.is_some() {
            ""
        } else {
            "width:max-content;"
        };
        format!("{width}z-index:{z};position:fixed;left:-10000px;top:0;visibility:hidden")
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
    //
    // 父层 id 在这里**只读一次**（untracked）：它晚于本层 effect 被写入是常态
    // （根菜单的 id 在自己的 effect 里才有），但不能因此让本层 effect 重跑。
    //
    // 关闭走两步：先把本层标成 closing（面板留着、栈上留着），EXIT_MS 之后才出栈。
    // 「先入栈、再请人」的顺序不受影响 —— 仲裁在 `push_child` 里同步完成，这里只推迟出栈。
    Effect::new(move |_| {
        if visible.get() {
            // 退场期间又可见（鼠标扫回来 / 重开）：作废那张票，面板原地回到 open 态。
            gate.cancel();
            closing.set(false);
            if layer.get_untracked().is_none() {
                let parent = parent_layer.and_then(|p| p.get_untracked());
                let id = host.push_child(
                    OverlayKind::Menu,
                    parent,
                    Callback::new(move |_| set_visible.run(false)),
                );
                layer.set(Some(id));
            }
        } else if let Some(id) = layer.get_untracked() {
            if closing.get_untracked() {
                // Esc 连按：这一层已经在退场，绝不能再挂第二个出栈定时器。
                return;
            }
            closing.set(true);
            // 退场层不再接 Esc：连按两下要关两层，不能都打在这一层上。
            host.mark_closing(id);
            gate.arm().then(move || {
                closing.try_set(false);
                host.remove(id);
                if layer.try_get_untracked() == Some(Some(id)) {
                    layer.try_set(None);
                }
            });
        }
    });

    on_cleanup(move || {
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    // Portal 的 children 必须是可重复调用的闭包，而 `AnyView` 不是 Copy：
    // 把内容放进 StoredValue，挂载时取走一次（与 Popover 的 children 同一个处理）。
    let body_store = StoredValue::new_local(Some(body));
    // Portal 的 children 闭包会按 move 捕获，属性里用的是另一份；`owner` 后面
    // 还要传给子面板（同一串菜单靠它被点外判定认出来）。
    let owner_attr = owner.clone();
    // 空 label 不在这里兜底：属性渲染时再查文案表，切语言才能立刻反映到 aria-label。
    // 存成 `StoredValue`（Copy）而不是闭包：Portal 的 children 必须是 `Fn`，
    // 捕获一个非 Copy 的闭包会把整块视图退化成 `FnOnce`。
    let label_store = StoredValue::new_local(label);

    let panel = view! {
        <Portal>
            <div
                class="c-menu"
                role="menu"
                aria-label=move || {
                    let own = label_store.get_value();
                    if own.is_empty() {
                        t().menu_label.to_string()
                    } else {
                        own
                    }
                }
                tabindex="-1"
                data-menu-owner=owner_attr.clone()
                data-glass=glass
                // 退场态给 CSS 用（真正收起看栈句柄）。
                data-state=move || {
                    if closing.get() {
                        Some("closing")
                    } else if layer.get().is_some() {
                        Some("open")
                    } else {
                        None
                    }
                }
                node_ref=panel_ref
                style=panel_style
                on:keydown=on_key
            >
                {body_store.try_update_value(|b| b.take()).flatten()}
            </div>
        </Portal>
    }
    .into_any();

    // 子菜单面板与父面板并列（各自 Portal 到 body，彼此是兄弟）。
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
            Callback::new(move |v: bool| {
                if !v {
                    // 关掉这一层时顺手取消还没触发的悬停计时器：否则 300ms 前挂上的
                    // 那个回调会在 Esc / 点父项关掉之后把子菜单又打开一次。
                    if let Some(id) = hover_timer.get_value() {
                        if let Some(w) = web_sys::window() {
                            w.clear_timeout_with_handle(id);
                        }
                        hover_timer.set_value(None);
                    }
                }
                link.open.set(v);
            }),
            owner.clone(),
            link.panel,
            None,
            item.label.clone(),
            // 下一层挂在**本层**下面：这样它入栈时不会把本层请出。
            Some(layer),
        );
        parts.push(child);

        // 子面板定位：锚父项、贴右侧、放不下翻左。锚点是**父菜单项**，
        // 而父项完全可能在滚动后换了位置，所以还要跟随。
        let child_panel = link.panel;
        let child_anchor = link.parent_el;
        let child_focus = link.focus_moved;
        let follow = StoredValue::new_local(None::<SubFollow>);
        Effect::new(move |_| {
            if !child_visible.get() {
                follow.set_value(None);
                return;
            }
            let (Some(anchor_el), Some(panel)) =
                (child_anchor.get_value(), child_panel.get())
            else {
                return;
            };
            schedule(move || {
                if !child_visible.get_untracked() {
                    return;
                }
                if !anchor_el.is_connected() || !panel.is_connected() {
                    return;
                }
                let spec = sub_spec();
                let (placed, dir) = place_sub(&anchor_el, &panel, spec);
                anchor::apply(&panel, placed, spec);
                let _ = panel.set_attribute("data-sub-side", dir);
                if child_focus.get_value() {
                    focus_first_item(&panel);
                }
                let on_gone: Rc<dyn Fn()> = Rc::new(move || {
                    if child_visible.get_untracked() {
                        link.open.set(false);
                    }
                });
                follow.set_value(Some(follow_sub(
                    anchor_el,
                    panel.into(),
                    on_gone,
                )));
            });
        });

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
    }
    parts.into_iter().collect_view().into_any()
}

/// 根面板的锚点来源。
#[derive(Clone, Copy)]
enum TriggerSource {
    /// 调用方直接给触发器元素（`MenuButton`）。
    Element(NodeRef<html::Button>),
    /// 公共 `Menu`：调用方只给坐标，锚点取「打开瞬间的焦点元素」；连焦点都没有时
    /// 才把坐标落成一个幽灵锚点。三条路都走同一套 `base::anchor`。
    Coords(MaybeProp<(f64, f64, f64)>),
}

/// 坐标 API 的兜底锚点。
///
/// 公共 `Menu` 只给 `(left, top, width)`：正常情况锚点是「打开瞬间的焦点元素」，
/// 但程序化打开时可能一个焦点元素都没有。这时临时插一个零尺寸锚点，让坐标也走
/// 同一条 `base::anchor` 通道（翻转、夹取、跟随规则完全一致）。
/// 不进视图树，是因为绝大多数菜单用不到它 —— 不该为此在 body 里常驻一个空 div。
fn ghost_anchor(coords: (f64, f64, f64)) -> Option<web_sys::HtmlElement> {
    let doc = web_sys::window()?.document()?;
    let el = doc.create_element("div").ok()?;
    let (left, top, width) = coords;
    let _ = el.set_attribute("aria-hidden", "true");
    let _ = el.set_attribute("data-gd-anchor-ghost", "");
    let html: web_sys::HtmlElement = el.dyn_into().ok()?;
    let style = html.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("left", &format!("{left}px"));
    // 调用方给的 top 是「面板上边缘」：锚点的下边缘要先减掉缝，`place` 再加回来。
    let _ = style.set_property("top", &format!("{}px", top - TRIGGER_GAP));
    let _ = style.set_property("width", &format!("{width}px"));
    let _ = style.set_property("height", "0px");
    let _ = style.set_property("visibility", "hidden");
    let _ = style.set_property("pointer-events", "none");
    doc.body()?.append_child(html.unchecked_ref::<web_sys::Node>()).ok()?;
    Some(html)
}

/// 移除坐标兜底锚点：关闭与卸载都走这里，body 上不会留下空节点。
fn drop_ghost(ghost: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>) {
    if let Some(el) = ghost.get_value() {
        if let Some(parent) = el.parent_element() {
            let _ = parent.remove_child(el.unchecked_ref::<web_sys::Node>());
        }
        ghost.set_value(None);
    }
}

/// 根菜单的核心：Portal + 定位 + 点外关闭 + 焦点归还。
///
/// 面板挂到 body 之后，`position:fixed` 才是真的视口坐标 —— 祖先上的
/// transform / backdrop-filter 不再是包含块。
fn menu_core(
    items: Vec<MenuItem>,
    open: Signal<bool>,
    trigger: TriggerSource,
    label: String,
    glass: Option<&'static str>,
    on_select: Option<Callback<String>>,
    on_close: Option<Callback<()>>,
    // 打开后是否把焦点放进第一项。坐标菜单（右键）必须为 false：
    // 一打开就有一项是 focus 态，而 `.c-menu__item:focus` 与 `:hover`
    // 用的是同一套强调色 —— 看起来就像「还没指过去，第一项已经选中了」。
    focus_first: bool,
) -> AnyView {
    let host = OverlayHost::use_or_provide();
    let panel_ref = NodeRef::<html::Div>::new();
    let owner = next_id("gd-menu");
    let return_to = StoredValue::new_local(None::<web_sys::HtmlElement>);
    // 打开瞬间的触发器：点外判定与「跟随锚点」都要用它 —— 打开之后焦点已经在菜单里，
    // 现查 activeElement 只会查到菜单项自己。
    let trigger_el = StoredValue::new_local(None::<web_sys::HtmlElement>);
    // 坐标兜底锚点（只在「没有触发器元素」时才会被建出来），关闭时删掉。
    let ghost = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
    // 跟随句柄：关闭时 take 掉，Drop 自动解绑滚动 / resize / 尺寸监听。
    let watch = StoredValue::new_local(None::<AnchorWatch>);
    // 关闭时是否归还焦点。点外关闭不归还（用户已经去别处了），Esc / Tab / 选中都归还。
    let restore = StoredValue::new_local(true);

    let is_open = move || open.get();
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
        owner.clone(),
        panel_ref,
        glass,
        label,
        // 根菜单是独立浮层：入栈时会请出别的菜单（同 rank 仲裁）。
        None,
    );

    Effect::new(move |_| {
        if is_open() {
            let el: Option<web_sys::HtmlElement> = match trigger {
                TriggerSource::Element(r) => r.get().map(|b| b.into()),
                TriggerSource::Coords(_) => active_element(),
            };
            trigger_el.set_value(el);
            // 打开前的焦点就是「触发器」：菜单不是模态，归还只能自己做。
            // 只在第一次入栈时记录 —— effect 若因 anchor 变化重跑，焦点已经在菜单里了。
            if return_to.get_value().is_none() {
                return_to.set_value(active_element());
            }
            restore.set_value(true);
            // 点外关闭：面板已经 Portal 到 body，与触发器、子菜单是**三棵互不相交**的树，
            // 所以要分别认。文档级 pointerdown 比全屏遮罩稳 —— 遮罩会盖住触发器，
            // 用户点触发器想收起时会先关再开，看起来像「点了个寂寞」。
            if outside.with_value(|v| v.is_none()) {
                let sel = format!("[data-menu-owner='{owner}']");
                let panel_key = panel_ref;
                let trigger_key = trigger_el;
                if let Some(listener) = listen_document("pointerdown", move |ev| {
                    let Some(el) = ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    else {
                        return;
                    };
                    let node: &web_sys::Node = el.unchecked_ref();
                    // 面板子树 ∪ 子菜单子树：每层 `.c-menu`（不管多深）都带同一个
                    // owner 标记，一次 closest 就能把整串认出来。点菜单项不能关整串。
                    let in_menu = panel_key
                        .get()
                        .map(|p| p.contains(Some(node)))
                        .unwrap_or(false)
                        || el.closest(&sel).ok().flatten().is_some();
                    if in_menu {
                        return;
                    }
                    // 触发器子树：点它是「二次点击收起」，焦点本来就在它身上。
                    let in_trigger = trigger_key
                        .get_value()
                        .map(|t| t.contains(Some(node)))
                        .unwrap_or(false);
                    if in_trigger {
                        close_all.run(());
                        return;
                    }
                    restore.set_value(false);
                    close_all.run(());
                }) {
                    outside.set_value(Some(listener));
                }
            }
            // 等一帧：面板的 style 闭包（`left:-10000px;visibility:hidden`）先落定，
            // 这时才量得到真实宽高，也才轮得到 `apply` 写坐标。
            schedule(move || {
                if !is_open() {
                    return;
                }
                let Some(panel) = panel_ref.get() else { return };
                // 触发器 > 焦点元素 > 坐标幽灵锚点。
                let mut anchor_el = trigger_el.get_value();
                if anchor_el.is_none() {
                    anchor_el = ghost.get_value();
                }
                if anchor_el.is_none() {
                    let coords = match trigger {
                        TriggerSource::Coords(c) => c.get().unwrap_or((0.0, 0.0, 0.0)),
                        TriggerSource::Element(_) => return,
                    };
                    anchor_el = ghost_anchor(coords);
                    ghost.set_value(anchor_el.clone());
                }
                let Some(anchor_el) = anchor_el else { return };
                if !anchor_el.is_connected() || !panel.is_connected() {
                    return;
                }
                let spec = root_spec();
                anchor::place_and_apply(&anchor_el, &panel, spec);
                if focus_first {
                    focus_first_item(&panel);
                } else {
                    // 焦点给面板本身：没有哪一项是 focus 态，
                    // 但键盘仍然可用（方向键 / Home / End 从面板出发）。
                    let _ = panel.focus();
                }
                // 锚点被滚出视口就收起：留一个挂在屏幕外的面板没有意义。
                // 这条关闭不该抢焦点（触发器此时在视口外）。
                let on_gone: Rc<dyn Fn()> = Rc::new(move || {
                    restore.set_value(false);
                    close();
                });
                let anchor_for_watch: web_sys::Element = anchor_el.into();
                watch.set_value(Some(anchor::watch(
                    anchor_for_watch,
                    panel.into(),
                    spec,
                    Some(on_gone),
                )));
            });
        } else {
            outside.set_value(None);
            watch.set_value(None);
            // 归还焦点：只有「这次关闭该还」且焦点没被别人接住才动。
            // 用标志位而不是查 activeElement —— 此刻面板可能已经被 display:none，
            // 浏览器已经把焦点丢回 body，查 DOM 永远得到「不在里面」。
            // 但「焦点是不是已经在别的菜单里」必须查 DOM：同 rank 仲裁关掉本层时，
            // 新菜单会在同一帧把焦点搬进自己，再把焦点抢回来就等于让键盘失效。
            if restore.get_value() {
                if let Some(el) = return_to.get_value() {
                    schedule(move || {
                        let own = panel_ref.get().map(|p| p.unchecked_into::<web_sys::Node>());
                        if el.is_connected() && focus_is_free(own.as_ref()) {
                            let _ = el.focus();
                        }
                    });
                }
            }
            restore.set_value(true);
            return_to.set_value(None);
            trigger_el.set_value(None);
            drop_ghost(ghost);
        }
    });

    on_cleanup(move || {
        watch.set_value(None);
        drop_ghost(ghost);
    });

    panel
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
    /// 打开后把焦点放进第一项。默认 true（键盘流）；
    /// 右键菜单传 false —— 菜单不该在指针还没指过去时就有一项亮着。
    #[prop(default = true)] focus_first: bool,
) -> impl IntoView {
    // 空 label 不在这里兜底：面板渲染时才查文案表，切语言能立刻反映到 aria-label。
    menu_core(
        items,
        Signal::derive(move || open.get().unwrap_or(false)),
        TriggerSource::Coords(anchor),
        label,
        fixed_size.then_some("menu-240x280"),
        on_select,
        on_close,
        focus_first,
    )
}

/// 触发器 + 菜单。触发器自己就是锚点（`NodeRef` 交给 Menu），所以调用方只要给条目。
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
    let trigger_ref = NodeRef::<html::Button>::new();
    // 记录「pointerdown 时菜单已经开着」：Menu 的点外监听会先把菜单关掉，
    // 紧接着的 click 若照常 toggle 就会立刻又打开 —— 看上去像没反应。
    let pressed_open = StoredValue::new_local(false);
    let items_for_menu = items.clone();
    let menu_label = label.clone();

    let mut cls = String::from("c-btn");
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let open_menu = move || {
        // 先确保焦点在触发器上：菜单关闭时要靠 activeElement 归还焦点，
        // 而程序化打开（或某些指针设备）不一定会自动聚焦按钮。
        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
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
        {menu_core(
            items_for_menu,
            Signal::derive(move || open.get()),
            TriggerSource::Element(trigger_ref),
            menu_label,
            fixed_size.then_some("menu-240x280"),
            on_select,
            Some(Callback::new(move |_| open.set(false))),
            // 按钮触发的菜单是键盘流：打开就该能直接 ↓ 选，第一项带 focus。
            true,
        )}
    }
}
