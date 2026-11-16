//! 统一浮层栈：Esc、z-index、模态 inert、同 rank 仲裁与焦点归还的唯一权威。
//!
//! 为什么需要它：Dialog 与 Select 各写一套 z-index 与关闭逻辑，两层以上就会互相压；
//! Dialog 也没有焦点陷阱。这里用 Leptos context 提供**一个**栈（不是全局单例），
//! 组件只入栈/出栈，不再自己决定「谁在最上面」。
//!
//! 两条规则让「同时开着两个菜单」不可能发生：
//!
//! 1. **lineage**：子菜单入栈时声明父层（`push_child`），父层永远不会被自己的子层请出；
//! 2. **仲裁**：新层入栈时，祖先链以外的**非模态**层全部收一遍。模态层不动 ——
//!    菜单不该因为弹了个对话框就被别人关掉，对话框之间也不互相仲裁。

use std::rc::Rc;

use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// 浮层种类。决定是否模态，以及同一栈深下的排序权重。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OverlayKind {
    Menu,
    Popover,
    Tooltip,
    Dialog,
    Sheet,
    Drawer,
}

impl OverlayKind {
    /// 模态：打开时背景 inert、焦点进入容器、关闭归还焦点。
    pub const fn is_modal(self) -> bool {
        matches!(self, Self::Dialog | Self::Sheet | Self::Drawer)
    }

    /// kind 的层基线。栈深每加一层是 +100，所以这里的 0/1/2 只用来打破同深度
    /// （同深度本来就不会出现两个浮层，留作语义区分）。
    pub const fn base_z(self) -> i32 {
        Z_BASE + self.rank()
    }

    const fn rank(self) -> i32 {
        match self {
            Self::Dialog | Self::Sheet | Self::Drawer => 0,
            Self::Menu | Self::Popover => 1,
            Self::Tooltip => 2,
        }
    }
}

/// 浮层栈的 z 起点：顶栏 10、菜单 400、对话框遮罩 500 都在此之下。
const Z_BASE: i32 = 1000;
/// 每深一层 +100，保证「后打开的永远压住先打开的」。
const Z_STEP: i32 = 100;

/// 提示条不参与 Esc 栈，但必须压在所有浮层之上，否则会被模态遮罩压暗。
pub const Z_TOAST: i32 = 4000;

/// 一次入栈的句柄。`remove` 是幂等的，句柄可以安全地留在 signal 里。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OverlayId(u32);

#[derive(Clone)]
struct Entry {
    id: OverlayId,
    kind: OverlayKind,
    /// 这条浮层挂在谁下面：子菜单挂在根菜单下，`None` 表示独立浮层。
    /// 仲裁靠它区分「我的父层」与「无关浮层」—— 认不出父层就会把父菜单一起关掉。
    parent: Option<OverlayId>,
    on_dismiss: Callback<()>,
    /// 焦点陷阱的边界（只对模态有意义）。
    container: Option<web_sys::HtmlElement>,
    /// 打开前的焦点，关闭后归还。
    return_to: Option<web_sys::HtmlElement>,
    /// 只聚焦一次：Leptos 重渲染会反复触发绑定，不能每次都抢焦点。
    focused: bool,
}

/// 浮层栈。`OverlayHost::use_or_provide()` 取；`AppShell` 已经在根上 provide 一次。
///
/// 所有字段都是 `Copy`（DOM 节点存在 LocalStorage 的 signal 里），所以组件可以
/// 把 host 直接塞进任意闭包，不用 `Rc` / `StoredValue` 包一层。
#[derive(Clone, Copy)]
pub struct OverlayHost {
    entries: RwSignal<Vec<Entry>, LocalStorage>,
    next: RwSignal<u32>,
    /// 只用于持有监听器：owner 释放时 Drop 会 removeEventListener。
    _esc: StoredValue<Option<Rc<EscListener>>, LocalStorage>,
}

impl OverlayHost {
    /// 在应用根调用一次。之后所有浮层共用这一条栈。
    pub fn provide() -> Self {
        let entries = RwSignal::new_local(Vec::new());
        let host = Self {
            entries,
            next: RwSignal::new(0),
            _esc: StoredValue::new_local(install_listener(entries)),
        };
        provide_context(host);
        host
    }

    /// 组件里的标准写法：有就用，没有就自己 provide（用于没套 AppShell 的页面）。
    pub fn use_or_provide() -> Self {
        use_context::<Self>().unwrap_or_else(Self::provide)
    }

    /// 入栈一条**独立**浮层（菜单 / 气泡 / 对话框）。返回的句柄交给 `remove`。
    ///
    /// 独立层会请出栈上其它非模态层：两个菜单不可能同时开着。
    pub fn push(&self, kind: OverlayKind, on_dismiss: Callback<()>) -> OverlayId {
        self.push_child(kind, None, on_dismiss)
    }

    /// 入栈一条**挂在 `parent` 下面**的浮层（目前只有子菜单用）。
    ///
    /// `parent` 是「我的容器」而不是「我的兄弟」：祖先链上的层不会被请出，
    /// 所以打开子菜单不会把父菜单收掉；而另一个根菜单仍然会被收掉。
    pub fn push_child(
        &self,
        kind: OverlayKind,
        parent: Option<OverlayId>,
        on_dismiss: Callback<()>,
    ) -> OverlayId {
        // 仲裁要在入栈前算：算的是「谁挡在前面」，新层自己不该参与。
        let keep = parent
            .map(|p| self.entries.with_untracked(|list| chain(list, p)))
            .unwrap_or_default();
        let vacated = self
            .entries
            .with_untracked(|list| vacate_targets(list, &keep));

        let id = OverlayId(self.next.get_untracked() + 1);
        self.next.set(id.0);
        // 记录打开前的焦点：必须在 DOM 变动之前取，晚了就是容器自己。
        let return_to = kind.is_modal().then(active_element).flatten();
        self.entries.update(|list| {
            list.push(Entry {
                id,
                kind,
                parent,
                on_dismiss,
                container: None,
                return_to,
                focused: false,
            });
        });

        // 先入栈、再请人：被请出的那一层关闭时要判断「焦点有没有被别处接住」，
        // 它必须已经能在栈上看见新层，否则会把焦点抢回自己的触发器。
        for cb in vacated {
            cb.run(());
        }
        id
    }

    /// 把焦点容器交给栈（模态必调）。第一次绑定时上 inert 并把焦点送进容器。
    pub fn bind(&self, id: OverlayId, container: web_sys::HtmlElement) {
        let mut first_modal = false;
        self.entries.update(|list| {
            if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
                if !entry.focused {
                    entry.focused = true;
                    first_modal = entry.kind.is_modal();
                }
                entry.container = Some(container.clone());
            }
        });
        if first_modal {
            set_inert(&container, true);
            let el = container.clone();
            // 等一帧：Leptos 的声明式重建还没落定时，focus() 会被后续 DOM 操作顶掉。
            // 期间若已经关闭，就别再把焦点塞进 display:none 的容器。
            let host = *self;
            schedule(move || {
                if host.is_open(id) {
                    focus_first(&el);
                }
            });
        }
    }

    /// 出栈。幂等；模态层会恢复 inert 与焦点。
    pub fn remove(&self, id: OverlayId) {
        let entry = self
            .entries
            .with_untracked(|list| list.iter().find(|e| e.id == id).cloned());
        let Some(entry) = entry else {
            return;
        };
        self.entries.update(|list| list.retain(|e| e.id != id));
        if !entry.kind.is_modal() {
            return;
        }
        if let Some(container) = &entry.container {
            set_inert(container, false);
        }
        if let Some(el) = entry.return_to {
            // 归还也在下一帧：先让关闭动画/display:none 落定，避免刚 focus 就被移除。
            schedule(move || {
                if el.is_connected() {
                    let _ = el.focus();
                }
            });
        }
    }

    pub fn is_open(&self, id: OverlayId) -> bool {
        self.entries.with_untracked(|list| list.iter().any(|e| e.id == id))
    }

    pub fn is_top(&self, id: OverlayId) -> bool {
        self.entries.with_untracked(|list| list.last().map(|e| e.id) == Some(id))
    }

    /// z-index 由栈推导，组件不再写 899/900 这类魔法数。
    pub fn z(&self, id: OverlayId) -> i32 {
        self.entries.with_untracked(|list| {
            list.iter()
                .position(|e| e.id == id)
                .map(|idx| Z_BASE + idx as i32 * Z_STEP + list[idx].kind.rank())
                .unwrap_or(Z_BASE)
        })
    }

    /// 模态层数（不是全部层数）：供 inert / 滚动锁判断。
    pub fn depth(&self) -> usize {
        self.entries.with_untracked(|list| list.iter().filter(|e| e.kind.is_modal()).count())
    }

    pub fn has_modal(&self) -> bool {
        self.depth() > 0
    }

    /// Esc / 点外调用。只碰最上层，不递归。
    pub fn dismiss_top(&self) {
        let cb = self.entries.with_untracked(|list| list.last().map(|e| e.on_dismiss));
        if let Some(cb) = cb {
            cb.run(());
        }
    }
}

/// 一条浮层的祖先链，从根排到自己（含自己）。
///
/// 环防御：父链最多和栈一样长 —— `parent` 都是比自己早入栈的 id，
/// 正常情况下爬不到一半就断了，这里只是不让未来的改动把宿主挂死。
fn chain(list: &[Entry], id: OverlayId) -> Vec<OverlayId> {
    let mut out = Vec::new();
    let mut cur = Some(id);
    while let Some(now) = cur {
        if out.contains(&now) {
            break;
        }
        out.push(now);
        cur = list.iter().find(|e| e.id == now).and_then(|e| e.parent);
    }
    out.reverse();
    out
}

/// 仲裁名单：`keep`（新层的祖先链）以外的非模态层，全部请出。
///
/// 只取每棵无关子树的**根**：父层收起会顺着信号带着子层一起收，
/// 逐个回调会让同一个组件在关闭过程中被改两次状态。
fn vacate_targets(list: &[Entry], keep: &[OverlayId]) -> Vec<Callback<()>> {
    let doomed: Vec<OverlayId> = list
        .iter()
        .filter(|e| !e.kind.is_modal() && !keep.contains(&e.id))
        .map(|e| e.id)
        .collect();
    if doomed.is_empty() {
        return Vec::new();
    }
    list.iter()
        .filter(|e| doomed.contains(&e.id))
        .filter(|e| {
            // 祖先也在这批里 → 交给祖先收，自己不再单独回调。
            chain(list, e.id)
                .iter()
                .all(|a| *a == e.id || !doomed.contains(a))
        })
        .map(|e| e.on_dismiss)
        .collect()
}

/// 归还焦点之前的最后一问：焦点是不是已经被别处接住。
///
/// 判定必须在**真正要 focus 的那一帧**做，而且要看 DOM 而不是看栈：
/// 被仲裁关掉的那一层，新层早已入栈，但焦点有没有搬进去要到下一帧才知道。
/// 自己的面板此时通常已经 `display:none`，浏览器把焦点丢回 body —— 那才算「空着」。
///
/// `own` 给不出（面板已卸载）时按「空着」处理：那是升级前的老行为，不能因为
/// 拿不到面板就把焦点永久丢掉。
pub(crate) fn focus_is_free(own: Option<&web_sys::Node>) -> bool {
    let Some(own) = own else {
        return true;
    };
    let Some(active) = active_element() else {
        return true;
    };
    let tag = active.tag_name();
    if tag.eq_ignore_ascii_case("body") || tag.eq_ignore_ascii_case("html") {
        return true;
    }
    // 焦点还在自己面板里（含面板本身）→ 可以还给触发器；已经在别处 → 别碰。
    own.contains(Some(active.unchecked_ref()))
}

/// 文档级 keydown：Esc 只关最上层，Tab 在模态容器内循环。
struct EscListener {
    target: web_sys::EventTarget,
    closure: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

impl Drop for EscListener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback("keydown", self.closure.as_ref().unchecked_ref());
    }
}

fn install_listener(entries: RwSignal<Vec<Entry>, LocalStorage>) -> Option<Rc<EscListener>> {
    let window = web_sys::window()?;
    let doc = window.document()?;
    let target: web_sys::EventTarget = doc.clone().into();

    let closure = Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
        // 输入法组合态里的 Esc 是「取消候选」，不是关浮层。
        if ev.is_composing() {
            return;
        }
        match ev.key().as_str() {
            "Escape" => {
                let top = entries.with_untracked(|list| list.last().map(|e| e.on_dismiss));
                if let Some(cb) = top {
                    ev.prevent_default();
                    cb.run(());
                }
            }
            "Tab" => {
                if trap_tab(&entries, ev.shift_key()) {
                    ev.prevent_default();
                }
            }
            _ => {}
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    let _ = target.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
    Some(Rc::new(EscListener { target, closure }))
}

/// 焦点陷阱。返回 true 表示已经处理过 Tab（调用方要 preventDefault）。
fn trap_tab(entries: &RwSignal<Vec<Entry>, LocalStorage>, shift: bool) -> bool {
    let Some(container) = entries.with_untracked(|list| {
        list.last()
            .filter(|e| e.kind.is_modal())
            .and_then(|e| e.container.clone())
    }) else {
        return false;
    };

    let items = focusables(&container);
    let Some(first) = items.first() else {
        let _ = container.focus();
        return true;
    };
    let last = items.last().unwrap_or(first);
    let active = active_element();
    let inside = active
        .as_ref()
        .map(|el| container.contains(Some(el.unchecked_ref::<web_sys::Node>())))
        .unwrap_or(false);

    let at_first = active
        .as_ref()
        .map(|el| el.is_same_node(Some(first.unchecked_ref::<web_sys::Node>())))
        .unwrap_or(false);
    let at_last = active
        .as_ref()
        .map(|el| el.is_same_node(Some(last.unchecked_ref::<web_sys::Node>())))
        .unwrap_or(false);

    if shift {
        if !inside || at_first {
            let _ = last.focus();
            return true;
        }
    } else if !inside || at_last {
        let _ = first.focus();
        return true;
    }
    false
}

fn focusables(root: &web_sys::HtmlElement) -> Vec<web_sys::HtmlElement> {
    let mut out = Vec::new();
    let Ok(list) = root.query_selector_all(
        "button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), \
         textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
    ) else {
        return out;
    };
    for i in 0..list.length() {
        let Some(node) = list.item(i) else { continue };
        let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() else {
            continue;
        };
        if el.get_attribute("aria-hidden").as_deref() != Some("true") {
            out.push(el);
        }
    }
    out
}

fn focus_first(container: &web_sys::HtmlElement) {
    match focusables(container).into_iter().next() {
        Some(first) => {
            let _ = first.focus();
        }
        None => {
            // 容器自己兜底（Dialog 面板已经带 tabindex="-1"，CSS 里也去了焦点环）。
            let _ = container.focus();
        }
    }
}

pub(crate) fn active_element() -> Option<web_sys::HtmlElement> {
    let doc = web_sys::window()?.document()?;
    let el = doc.active_element()?;
    // body 不算「触发器」：程序化打开时没有可归还的目标。
    if el.tag_name().eq_ignore_ascii_case("body") {
        return None;
    }
    el.dyn_into::<web_sys::HtmlElement>().ok()
}

/// 给背景加 inert。
///
/// Dialog 挂在页面内容里（不是 Portal），直接 inert `.l-app` 会把它自己也冻住。
/// 所以逐级向上，把**每一层祖先的兄弟节点**标 inert —— 祖先链本身保持可交互，
/// 容器所在的那条路径就是唯一活的区域。计数支持嵌套模态，不会整片清除。
fn set_inert(container: &web_sys::HtmlElement, on: bool) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let app = doc.query_selector(".l-app").ok().flatten();
    let container_node: web_sys::Node = container.clone().into();

    let mut node = container_node.clone();
    loop {
        let Some(parent) = node.parent_node() else { break };
        let is_app = app
            .as_ref()
            .map(|a| parent.is_same_node(Some(a.unchecked_ref::<web_sys::Node>())))
            .unwrap_or(false);
        if let Some(parent_el) = parent.dyn_ref::<web_sys::Element>() {
            let children = parent_el.children();
            for i in 0..children.length() {
                let Some(child) = children.item(i) else { continue };
                if !child.is_same_node(Some(&node)) {
                    toggle_inert(&child, on);
                }
            }
        }
        // 到应用根（或 body）就停：再往上会碰到 document / html。
        if is_app || parent.node_name() == "BODY" {
            break;
        }
        node = parent;
    }

    // 容器不在 `.l-app` 里（将来挂到 body 的 Portal）：退回整块应用根。
    if let Some(app_el) = app {
        if !app_el.contains(Some(&container_node)) {
            toggle_inert(&app_el, on);
        }
    }
}

fn toggle_inert(el: &web_sys::Element, on: bool) {
    let count = el
        .get_attribute("data-gd-inert")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    if on {
        let next = count + 1;
        let _ = el.set_attribute("data-gd-inert", &next.to_string());
        let _ = el.set_attribute("inert", "");
    } else if count > 1 {
        let _ = el.set_attribute("data-gd-inert", &(count - 1).to_string());
    } else {
        let _ = el.remove_attribute("data-gd-inert");
        let _ = el.remove_attribute("inert");
    }
}

/// 下一帧执行。为什么浮层都要等一帧：Leptos 的声明式渲染还没落定时量到的
/// `offsetHeight` / `focus()` 会被后续 DOM 操作顶掉。
pub(crate) fn schedule(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}

/// 文档级监听器的句柄。Drop 时解绑，所以放进 `StoredValue` 就能活到组件卸载。
///
/// 非模态浮层（菜单 / 气泡 / 提示）不归 inert 与焦点陷阱管，点外关闭、滚动取消
/// 只能自己做。监听器统一走这里，避免每个组件各写一份 `Closure` 生命周期管理
/// 时忘记解绑 —— 泄漏的监听器会在浮层关掉之后继续改信号。
pub(crate) struct GlobalListener {
    target: web_sys::EventTarget,
    kind: &'static str,
    capture: bool,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for GlobalListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.kind,
            self.closure.as_ref().unchecked_ref(),
            self.capture,
        );
    }
}

/// 在 `target` 上挂一个监听器。`capture=true` 才能在容器滚动时先于冒泡收到（scroll 不冒泡）。
pub(crate) fn listen(
    target: web_sys::EventTarget,
    kind: &'static str,
    capture: bool,
    handler: impl FnMut(web_sys::Event) + 'static,
) -> Option<GlobalListener> {
    let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(web_sys::Event)>);
    target
        .add_event_listener_with_callback_and_bool(
            kind,
            closure.as_ref().unchecked_ref(),
            capture,
        )
        .ok()?;
    Some(GlobalListener {
        target,
        kind,
        capture,
        closure,
    })
}

/// 文档级监听（点外关闭）。
pub(crate) fn listen_document(
    kind: &'static str,
    handler: impl FnMut(web_sys::Event) + 'static,
) -> Option<GlobalListener> {
    let doc = web_sys::window()?.document()?;
    listen(doc.into(), kind, false, handler)
}

/// 窗口级监听。`capture=true` 时能收到内部滚动容器的事件（scroll 不冒泡）。
pub(crate) fn listen_window(
    kind: &'static str,
    capture: bool,
    handler: impl FnMut(web_sys::Event) + 'static,
) -> Option<GlobalListener> {
    let window = web_sys::window()?;
    listen(window.into(), kind, capture, handler)
}

/// 面板里第一个可聚焦元素。气泡 / 提示要把焦点交给内容和触发器时需要它。
pub(crate) fn first_focusable(root: &web_sys::HtmlElement) -> Option<web_sys::HtmlElement> {
    focusables(root).into_iter().next()
}

