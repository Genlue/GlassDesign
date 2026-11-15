//! 工具栏自定义 ToolbarCustomization —— 「有哪些项、按什么顺序」。
//!
//! 和 `ToolbarOverflow` 的分工：溢出是**宽度算出来的**（放不下就收进菜单），
//! 自定义是**应用设置**（用户说了算，必须显式保存）。两件事混在一个控件里，
//! 用户就分不清「我的按钮去哪了」到底是被收起了还是被关掉了。
//!
//! 为什么渲染的是「设置列表」而不是那块浮层面板（`.c-toolbarcustom` 是 fixed 的
//! 300px 浮层）：props 里没有 open / anchor，它没法自己决定什么时候出现。
//! 浮层外壳交给调用方（`Popover` 或自己的面板），这里只负责 `.c-toolbarcustom__body`
//! 以内的一切 —— 行、手柄、开关、「恢复默认」。这样同一份列表既能放进浮层，
//! 也能直接贴进设置页。
//!
//! 键盘契约（纯拖拽不可访问，所以必须有第二条路）：
//! - 手柄可聚焦，`Alt+↑ / ↓` 与相邻行换位；
//! - 手柄上 `Space` 切换该行显隐（手不用离开键盘去够右边的开关）；
//! - 开关自己就是原生 `button[role=switch]`，`Space` / `Enter` 由它接管。
//!
//! 锁定项（`locked`）是重排的硬边界：它自己不动、别人也不能跨过它，
//! 和 `DocumentTabs` 里 pinned 页签同一套约定。它的显示开关也点不动 ——
//! 「锁定的东西还能藏起来」等于没锁。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::base::overlay::{listen_window, schedule, GlobalListener};
use crate::widgets::switch::Switch;

/// 进入拖拽的阈值（px）。手按在手柄上总会有几像素抖动，
/// 阈值太小会把「按一下」读成「想拖」，行就莫名其妙挪了位。
const DRAG_THRESHOLD: f64 = 4.0;

/// 工具栏里可自定义的一项。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ToolbarField {
    pub id: String,
    pub label: String,
    pub visible: bool,
    /// 锁定：永远可见、不能隐藏、不参与重排。
    pub locked: bool,
}

impl ToolbarField {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            visible: true,
            locked: false,
        }
    }

    /// 默认不显示（开关初始是关的）。
    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    /// 锁定项（比如「撤销」这种不能没有的动作）。
    pub fn locked(mut self) -> Self {
        self.locked = true;
        self.visible = true;
        self
    }
}

/// 重画之后焦点该还给谁。重画会换掉整行节点，
/// 不还焦点的话键盘用户每次切换都在原地丢失位置。
#[derive(Clone, Copy)]
enum FocusOn {
    Grip,
    Switch,
}

#[derive(Clone, PartialEq, Debug)]
struct DragState {
    id: String,
    /// 按下时的指针 y。阈值判断只看纵向：列表是一维的。
    y0: f64,
    /// 哪根指针在拖。多指触摸时别让第二根手指接管。
    pointer: i32,
    started: bool,
}

/// 工具栏自定义列表。
///
/// `fields` 是**首屏快照**：顺序与显隐由组件自己维护，变更通过 `on_change` 报出去。
/// 这样调用方不必回灌一份新 `Vec`（回灌会让整个列表重建，焦点、指针捕获全丢）。
#[component]
pub fn ToolbarCustomization(
    #[prop(into)] fields: Vec<ToolbarField>,
    /// 非空则读写 `localStorage[key]`（JSON：`[{"id":"a","visible":true}]`，数组顺序即顺序）。
    #[prop(optional)] storage_key: String,
    #[prop(optional)] on_change: Option<Callback<Vec<ToolbarField>>>,
) -> impl IntoView {
    let root_ref = NodeRef::<html::Div>::new();
    // reset 的目标：入参的默认表。不能取「当前值」—— 否则「恢复默认」只是
    // 把用户改回上次保存的样子，看起来像没生效。
    let base = StoredValue::new_local(normalize(fields.clone()));
    let rows = RwSignal::new(initial_rows(fields, &storage_key));
    let key = StoredValue::new_local(storage_key);
    let drag = RwSignal::new(None::<DragState>);
    let drag_id = RwSignal::new(None::<String>);
    // 按下时的顺序：pointercancel 要能原地回滚。
    let drag_origin = StoredValue::new_local(Vec::<ToolbarField>::new());

    /* ---------------- 变更 ---------------- */

    // 所有「顺序 / 显隐」的变化都走这里：落盘 → 重画 → 回报 → 还焦点。
    // 先重画再回调，是因为回调里常常会立刻读一遍新顺序。
    let commit = move |next: Vec<ToolbarField>, focus: Option<(String, FocusOn)>| {
        write_saved(&key.get_value(), &next);
        rows.set(next.clone());
        if let Some(cb) = on_change {
            cb.run(next);
        }
        if let Some((id, which)) = focus {
            focus_row(root_ref, id, which);
        }
    };

    // `focus` 决定重画后焦点回到哪：从开关点进来留在开关上，
    // 从手柄按空格则留在手柄上（手不用来回跳）。
    let toggle = move |id: String, on: bool, focus: FocusOn| {
        let mut list = rows.get_untracked();
        let mut changed = false;
        if let Some(row) = list.iter_mut().find(|f| f.id == id) {
            if !row.locked && row.visible != on {
                row.visible = on;
                changed = true;
            }
        }
        if changed {
            commit(list, Some((id, focus)));
        }
    };

    // Alt+↑/↓：和相邻行换位。锁定项不参与，也不允许跨过它。
    let swap = move |id: String, step: isize| {
        let mut list = rows.get_untracked();
        let Some(i) = list.iter().position(|f| f.id == id) else {
            return;
        };
        let j = i as isize + step;
        if j < 0 || j >= list.len() as isize {
            return;
        }
        let j = j as usize;
        if list[i].locked || list[j].locked {
            return;
        }
        list.swap(i, j);
        commit(list, Some((id, FocusOn::Grip)));
    };

    let reset = move || {
        let next = base.get_value();
        // 恢复默认 = 回到入参顺序与显隐，并**清掉**存储。
        // 这里若再写一遍 storage，「默认」就成了一次用户修改，
        // 下次分不清「从没动过」和「恢复过默认」。
        clear_saved(&key.get_value());
        rows.set(next.clone());
        if let Some(cb) = on_change {
            cb.run(next);
        }
    };

    // 挂载后回报一次「解析后的状态」（含从 localStorage 恢复的顺序）：
    // 调用方要维护镜像（比如画廊右侧的计数与顺序）才拿得到正确初值。
    Effect::new(move |_| {
        if let Some(cb) = on_change {
            cb.run(rows.get_untracked());
        }
    });

    /* ---------------- 拖拽：监听挂在 window 上 ---------------- */

    // 为什么不用手柄上的 pointer capture：拖拽期间每换一次位都要按数据重建行，
    // 被按住的那个节点会被换掉，浏览器随之释放捕获（gotpointercapture 紧跟着
    // lostpointercapture）；即便只挪节点，按规范也会丢捕获。
    // 挂着 window 上，节点怎么换都不影响事件链。
    let drag_move = move |ev: web_sys::PointerEvent| {
        let Some(mut d) = drag.get_untracked() else {
            return;
        };
        if d.pointer != ev.pointer_id() {
            return;
        }
        if !d.started {
            if (ev.client_y() as f64 - d.y0).abs() < DRAG_THRESHOLD {
                return;
            }
            d.started = true;
            drag_id.set(Some(d.id.clone()));
            drag.set(Some(d.clone()));
        }
        if ev.cancelable() {
            ev.prevent_default();
        }
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        let root_el: &web_sys::Element = &root;

        // 行中点。DOM 顺序 == 数据顺序（拖拽期间只有数据在变），所以
        // 「第一个中点低于指针的行」就是插入位置。
        let y = ev.client_y() as f64;
        let mids: Vec<(String, f64)> = row_elements(root_el)
            .iter()
            .filter_map(|el| {
                let id = el.get_attribute("data-tf-id")?;
                let r = el.get_bounding_client_rect();
                Some((id, r.top() + r.height() / 2.0))
            })
            .collect();

        let list = rows.get_untracked();
        let Some(from) = list.iter().position(|f| f.id == d.id) else {
            return;
        };
        // 抹掉被拖项，剩下的是「候选落点序列」
        let mut others: Vec<ToolbarField> = Vec::with_capacity(list.len().saturating_sub(1));
        for (i, f) in list.iter().enumerate() {
            if i != from {
                others.push(f.clone());
            }
        }
        // 锁定行是硬边界：只能在本段（前后最近两个锁定项之间）内移动。
        let lo = list[..from]
            .iter()
            .rposition(|f| f.locked)
            .map(|p| p + 1)
            .unwrap_or(0);
        let hi = list[from + 1..]
            .iter()
            .position(|f| f.locked)
            .map(|p| from + p)
            .unwrap_or(others.len());
        let at = others
            .iter()
            .take_while(|f| {
                mids.iter()
                    .find(|(id, _)| id == &f.id)
                    .map(|(_, mid)| *mid < y)
                    .unwrap_or(false)
            })
            .count()
            .clamp(lo.min(others.len()), hi.min(others.len()));
        // 位置没变就别重建：每次 set 都会换掉整行 DOM。
        if at == from {
            return;
        }
        others.insert(at, list[from].clone());
        // 拖拽期间只改数据顺序，不写 storage、不回调 —— 松手才落盘。
        rows.set(others);
    };

    // `commit_order=false`（pointercancel）用起点顺序回滚。
    let finish_drag = move |commit_order: bool| {
        let Some(d) = drag.get_untracked() else {
            return;
        };
        drag.set(None);
        drag_id.set(None);
        if !d.started {
            return;
        }
        if commit_order {
            // 拖拽期间 rows 已经是最终顺序：这里补一次落盘 + 回报 + 还焦点。
            commit(rows.get_untracked(), Some((d.id.clone(), FocusOn::Grip)));
        } else {
            rows.set(drag_origin.get_value());
            focus_row(root_ref, d.id.clone(), FocusOn::Grip);
        }
    };

    let drag_end = move |ev: web_sys::PointerEvent| {
        let Some(d) = drag.get_untracked() else {
            return;
        };
        if d.pointer == ev.pointer_id() {
            finish_drag(true);
        }
    };
    let drag_cancel = move |ev: web_sys::PointerEvent| {
        let Some(d) = drag.get_untracked() else {
            return;
        };
        if d.pointer == ev.pointer_id() {
            finish_drag(false);
        }
    };

    let listeners = StoredValue::new_local(Vec::<GlobalListener>::new());
    {
        let mut list: Vec<GlobalListener> = Vec::new();
        if let Some(l) = listen_window("pointermove", false, move |ev| {
            if let Ok(ev) = ev.dyn_into::<web_sys::PointerEvent>() {
                drag_move(ev);
            }
        }) {
            list.push(l);
        }
        if let Some(l) = listen_window("pointerup", false, move |ev| {
            if let Ok(ev) = ev.dyn_into::<web_sys::PointerEvent>() {
                drag_end(ev);
            }
        }) {
            list.push(l);
        }
        if let Some(l) = listen_window("pointercancel", false, move |ev| {
            if let Ok(ev) = ev.dyn_into::<web_sys::PointerEvent>() {
                drag_cancel(ev);
            }
        }) {
            list.push(l);
        }
        listeners.set_value(list);
    }

    on_cleanup(move || {
        // 置空即 Drop，GlobalListener 的 Drop 负责 removeEventListener。
        listeners.set_value(Vec::new());
    });

    /* ---------------- 行 ---------------- */

    let row_view = move |f: ToolbarField| -> AnyView {
        let id = f.id.clone();
        let label = f.label.clone();
        let locked = f.locked;
        // 锁定项永远可见：数据里写了 false 也不认（存储被手改过也不该破契约）。
        let visible = f.visible || locked;

        let id_down = id.clone();
        let id_key = id.clone();
        let id_drag_attr = id.clone();
        let id_switch = id.clone();
        let switch_label = if locked {
            format!("{label}（锁定，不能隐藏）")
        } else {
            format!("显示 {label}")
        };
        // 锁定项的开关给一个「什么也不做」的回调：Switch 是受控组件，
        // 没有 on_change 就点不动，但 `#[prop(optional)]` 的 setter 只收
        // Callback 本体（不收 Option），所以这里统一给一个回调，
        // 真正挡住锁定项的是 `toggle` 里的 `locked` 判断。
        let on_switch = Callback::new(move |v: bool| toggle(id_switch.clone(), v, FocusOn::Switch));

        let grip = if locked {
            // 锁定行没有手柄按钮（不可拖），但仍占一个手柄的宽度：
            // 否则锁定行的文字会比可拖行往左偏 18px，列表左缘看起来是锯齿。
            view! {
                <span class="c-toolbarcustom__grip" aria-hidden="true">
                    <Icon name=IconName::GRIP size=10 />
                </span>
            }
            .into_any()
        } else {
            view! {
                <button
                    class="c-toolbarcustom__grip"
                    type="button"
                    aria-label=format!("拖动排序 {label}")
                    title="拖动排序（Alt+↑ / ↓ 换位，空格切换显示）"
                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                        if ev.button() != 0 {
                            return;
                        }
                        // 只记起点，move / up 交给 window 上的监听（见上方注释）。
                        ev.prevent_default();
                        drag_origin.set_value(rows.get_untracked());
                        drag.set(Some(DragState {
                            id: id_down.clone(),
                            y0: ev.client_y() as f64,
                            pointer: ev.pointer_id(),
                            started: false,
                        }));
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        let key = ev.key();
                        if ev.alt_key() && (key == "ArrowUp" || key == "ArrowDown") {
                            ev.prevent_default();
                            swap(id_key.clone(), if key == "ArrowDown" { 1 } else { -1 });
                            return;
                        }
                        if key == " " || key == "Spacebar" {
                            // 手柄兼一个便宜的显示开关：手不用离开键盘去够右边那颗。
                            ev.prevent_default();
                            let cur = rows
                                .get_untracked()
                                .iter()
                                .find(|f| f.id == id_key)
                                .map(|f| f.visible)
                                .unwrap_or(false);
                            toggle(id_key.clone(), !cur, FocusOn::Grip);
                        }
                    }
                >
                    <Icon name=IconName::GRIP size=10 />
                </button>
            }
            .into_any()
        };

        view! {
            <div
                class="c-toolbarcustom__row"
                role="listitem"
                data-tf-id=id.clone()
                data-locked=locked.then_some("1")
                data-dragging=move || {
                    (drag_id.get().as_deref() == Some(id_drag_attr.as_str())).then_some("1")
                }
                title=locked.then(|| "锁定项：不能隐藏，也不能排序".to_string())
            >
                {grip}
                <span class="c-toolbarcustom__text">{label.clone()}</span>
                {locked.then(|| view! { <span class="c-toolbarcustom__req">"锁定"</span> })}
                <Switch checked=visible label=switch_label on_change=on_switch />
            </div>
        }
        .into_any()
    };

    let list_view = move || {
        let all = rows.get();
        if all.is_empty() {
            return view! {
                <p class="c-toolbarcustom__empty">"没有可自定义的项"</p>
            }
            .into_any();
        }
        view! {
            <div class="c-toolbarcustom__list" role="list" aria-label="工具栏项">
                {all.into_iter().map(row_view).collect_view()}
            </div>
        }
        .into_any()
    };

    view! {
        <div class="c-toolbarcustom__body" node_ref=root_ref>
            <p class="c-toolbarcustom__hint">
                "勾选显示；拖左侧手柄排序，也可以用 Alt+↑ / ↓ 换位。"
            </p>
            {list_view}
            <div class="c-toolbarcustom__foot">
                <button class="c-toolbarcustom__reset" type="button" on:click=move |_| reset()>
                    "恢复默认"
                </button>
            </div>
        </div>
    }
}

/* ---------------- DOM 小工具 ---------------- */

fn row_elements(root: &web_sys::Element) -> Vec<web_sys::Element> {
    let mut out = Vec::new();
    let Ok(list) = root.query_selector_all("[data-tf-id]") else {
        return out;
    };
    for i in 0..list.length() {
        if let Some(node) = list.item(i) {
            if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                out.push(el);
            }
        }
    }
    out
}

fn row_el(root: &web_sys::Element, id: &str) -> Option<web_sys::Element> {
    row_elements(root)
        .into_iter()
        .find(|el| el.get_attribute("data-tf-id").as_deref() == Some(id))
}

/// 重画后把焦点还回去。必须等一帧：信号赋值那一刻新节点还没挂上，
/// 同步 `focus()` 会落在即将被丢弃的旧节点上。
fn focus_row(root: NodeRef<html::Div>, id: String, which: FocusOn) {
    schedule(move || {
        let Some(root) = root.get_untracked() else {
            return;
        };
        let root_el: &web_sys::Element = &root;
        let Some(row) = row_el(root_el, &id) else {
            return;
        };
        let sel = match which {
            FocusOn::Grip => ".c-toolbarcustom__grip",
            FocusOn::Switch => ".c-switch",
        };
        if let Ok(Some(el)) = row.query_selector(sel) {
            if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        }
    });
}

/* ---------------- 数据 ---------------- */

/// 去重 + 强制「锁定项可见」。
fn normalize(list: Vec<ToolbarField>) -> Vec<ToolbarField> {
    let mut out: Vec<ToolbarField> = Vec::with_capacity(list.len());
    for mut f in list {
        if f.id.is_empty() || out.iter().any(|o| o.id == f.id) {
            continue;
        }
        if f.locked {
            f.visible = true;
        }
        out.push(f);
    }
    out
}

/// 初始化：先按入参整理出默认表，再用存储里的顺序覆盖。
///
/// 存储里没提到的项（升级后新增的字段）按入参顺序补在末尾 —— 不能丢，
/// 否则用户升级一次就「少了几个按钮」，还得自己去找回来。
fn initial_rows(fields: Vec<ToolbarField>, key: &str) -> Vec<ToolbarField> {
    let defaults = normalize(fields);
    let saved = read_saved(key);
    if saved.is_empty() {
        return defaults;
    }
    let mut out: Vec<ToolbarField> = Vec::with_capacity(defaults.len());
    for (id, visible) in &saved {
        if out.iter().any(|o| &o.id == id) {
            continue;
        }
        if let Some(f) = defaults.iter().find(|f| &f.id == id) {
            let mut f = f.clone();
            f.visible = *visible || f.locked;
            out.push(f);
        }
    }
    for f in defaults {
        if !out.iter().any(|o| o.id == f.id) {
            out.push(f);
        }
    }
    out
}

/* ---------------- 存储：手写极简 JSON ----------------
   只处理 `[{"id":"…","visible":true}, …]` 这一种形状。
   引 serde 换来的是「多一个依赖 + 一截 wasm 体积」，而这里最多十几行、
   两个字段 —— 不划算。任何畸形输入都当作「没存过」：面板必须能打开。 */

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn read_saved(key: &str) -> Vec<(String, bool)> {
    if key.is_empty() {
        return Vec::new();
    }
    let Some(store) = storage() else {
        return Vec::new();
    };
    // 隐私模式 / 被策略禁用：读不到就当没存过。
    let Ok(Some(raw)) = store.get_item(key) else {
        return Vec::new();
    };
    parse_saved(&raw)
}

fn write_saved(key: &str, rows: &[ToolbarField]) {
    if key.is_empty() {
        return;
    }
    let Some(store) = storage() else {
        return;
    };
    // 配额满 / 无权限：本次会话仍然按内存里的新顺序走，静默放弃写入。
    let _ = store.set_item(key, &serialize(rows));
}

fn clear_saved(key: &str) {
    if key.is_empty() {
        return;
    }
    if let Some(store) = storage() {
        let _ = store.remove_item(key);
    }
}

fn parse_saved(raw: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        let obj = &rest[start + 1..start + len];
        rest = &rest[start + len + 1..];
        if let (Some(id), Some(visible)) = (json_string(obj, "id"), json_bool(obj, "visible")) {
            if !id.is_empty() {
                out.push((id, visible));
            }
        }
    }
    out
}

fn json_string(obj: &str, key: &str) -> Option<String> {
    let quoted = format!("\"{key}\"");
    let at = obj.find(&quoted)? + quoted.len();
    let after = obj[at..].trim_start().strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('"')?;
    let mut out = String::new();
    let mut esc = false;
    for ch in after.chars() {
        if esc {
            out.push(match ch {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            esc = false;
            continue;
        }
        match ch {
            '\\' => esc = true,
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
}

fn json_bool(obj: &str, key: &str) -> Option<bool> {
    let quoted = format!("\"{key}\"");
    let at = obj.find(&quoted)? + quoted.len();
    let after = obj[at..].trim_start().strip_prefix(':')?.trim_start();
    if after.starts_with("true") {
        Some(true)
    } else if after.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn serialize(rows: &[ToolbarField]) -> String {
    let body: Vec<String> = rows
        .iter()
        .map(|f| {
            format!(
                "{{\"id\":\"{}\",\"visible\":{}}}",
                json_escape(&f.id),
                f.visible
            )
        })
        .collect();
    format!("[{}]", body.join(","))
}
