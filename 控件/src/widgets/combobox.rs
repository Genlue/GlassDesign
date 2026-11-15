use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::base::id::next_id;
use crate::base::overlay::{OverlayHost, OverlayId, OverlayKind};

/// 组合框的一个候选。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComboboxOption {
    pub value: String,
    pub label: String,
    /// 右侧次要说明（城市码、数量…）。空字符串不渲染。
    pub hint: String,
    pub disabled: bool,
}

impl ComboboxOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            hint: String::new(),
            disabled: false,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

fn label_of(options: &[ComboboxOption], value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    options
        .iter()
        .find(|o| o.value == value)
        .map(|o| o.label.clone())
        // 允许自定义值时，值本身就是显示文本
        .unwrap_or_else(|| value.to_string())
}

/// 可输入 + 过滤列表的组合框。
///
/// 和 `Select` 只差一件事，但决定了整个实现：输入框里的文字是「查询」，
/// 选中的是「值」。两者分开存 —— 打字只改查询，确认后才回 `on_change`；
/// 合成一个字段就表达不了「打了字但还没选」这个中间态，`on_change` 还会每敲一键响一次。
///
/// 焦点始终留在输入框，候选只画高亮，用 `aria-activedescendant` 指过去。
/// 真把焦点移进列表会连带三个麻烦：输入法候选窗被顶掉、继续打字要先 Tab 回来、
/// 关闭后焦点归属还得单独记账。
///
/// 关闭有两条路：Esc（交给 `OverlayHost`，只关最上层）与焦点离开整个控件。
/// 点外不能用「全屏遮罩」那一招 —— 遮罩会盖住输入框，用户就点不回光标位置了；
/// 所以这里用 `focusout` 判「焦点是否还在包装器里」，候选行的 pointerdown
/// 已经 preventDefault，点候选不会被误判成点外。
#[component]
pub fn Combobox(
    #[prop(into)] options: Vec<ComboboxOption>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(optional)] allow_custom: bool,
    #[prop(into, optional)] placeholder: String,
    #[prop(into, optional)] label: String,
    #[prop(optional)] width: u32,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    let width = if width == 0 { 240 } else { width };
    let host = OverlayHost::use_or_provide();
    let panel_id = next_id("gd-combobox");
    // reveal 里要用 id 拼选择器，而它会被塞进多个闭包里 —— 存成 Copy 的句柄。
    let panel_key = StoredValue::new(panel_id.clone());
    let input_ref = NodeRef::<html::Input>::new();
    let wrap_ref = NodeRef::<html::Div>::new();
    let panel_ref = NodeRef::<html::Div>::new();

    let open = RwSignal::new(false);
    let layer = RwSignal::new(None::<OverlayId>);
    let pos = RwSignal::new((0.0f64, 0.0f64, 0.0f64));
    // 查询草稿：Some 时覆盖「已选值的标签」。只有面板开着才生效 ——
    // 关掉就回到已选值，Esc / 点外因此天然等于「放弃这次输入」。
    let draft = RwSignal::new(None::<String>);
    let active = RwSignal::new(None::<usize>);
    let options_for_text = options.clone();

    let text = Memo::new(move |_| {
        if let Some(d) = draft.get() {
            if open.get() {
                return d;
            }
        }
        label_of(&options_for_text, &value.get().unwrap_or_default())
    });

    let filtered = Memo::new({
        let options = options.clone();
        move |_| {
            // 过滤只看「正在敲的查询」，不看输入框里显示的已选值：
            // 刚选完再打开面板应该是完整列表，而不是只剩那一项。
            let q = draft.get().unwrap_or_default().trim().to_lowercase();
            options
                .iter()
                .filter(|o| q.is_empty() || o.label.to_lowercase().contains(&q))
                .cloned()
                .collect::<Vec<_>>()
        }
    });

    let focus_input = move || {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    // 关闭只有一条路：出栈 + 收起。点外、选中、Esc（host 回调）都走这里。
    let close = move || {
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
        open.set(false);
        active.set(None);
        // 收起草稿：输入框回到「已选值的标签」，下次打开是空查询（完整列表）
        draft.set(None);
    };

    let open_panel = move || {
        if disabled || layer.get_untracked().is_some() {
            return;
        }
        // 定位照抄 Select：读触发器矩形 → position:fixed。
        if let Some(el) = input_ref.get() {
            let r = el.get_bounding_client_rect();
            pos.set((r.left(), r.bottom() + 6.0, r.width()));
        }
        let id = host.push(
            OverlayKind::Popover,
            Callback::new(move |_| {
                close();
                // Esc 关掉之后焦点必须回到输入框：用户下一步就是接着打字。
                focus_input();
            }),
        );
        layer.set(Some(id));
        active.set(None);
        open.set(true);
    };

    // 点外关闭用 focusout，而不是文档级监听或全屏遮罩：
    // 遮罩会盖住输入框，用户就点不回光标位置；文档级监听又要自己管闭包的生命周期。
    // 候选行的 pointerdown 已经 preventDefault，所以点候选不会触发这里。
    on_cleanup(move || {
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    let layer_z = move || {
        layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Popover.base_z())
    };

    let active_index = move || active.get().filter(|i| *i < filtered.get().len());

    // 高亮跟键盘走：索引变化后把面板滚到能看见的位置。
    let reveal = move |index: usize| {
        let Some(panel) = panel_ref.get() else { return };
        let sel = format!("#{}-opt-{index}", panel_key.get_value());
        let Ok(Some(el)) = panel.query_selector(&sel) else {
            return;
        };
        let pr = panel.get_bounding_client_rect();
        let er = el.get_bounding_client_rect();
        if er.top() < pr.top() {
            panel.set_scroll_top(panel.scroll_top() - (pr.top() - er.top()).round() as i32);
        } else if er.bottom() > pr.bottom() {
            panel.set_scroll_top(panel.scroll_top() + (er.bottom() - pr.bottom()).round() as i32);
        }
    };

    // 循环移动并跳过禁用项：到底再按一次回到另一端，是 combo 的通行约定。
    let move_active = move |delta: i32| {
        let items = filtered.get();
        let n = items.len() as i32;
        if n == 0 {
            active.set(None);
            return;
        }
        let start = match active.get_untracked() {
            Some(i) if (i as i32) < n => i as i32,
            _ => {
                if delta > 0 {
                    -1
                } else {
                    n
                }
            }
        };
        let mut at = start;
        for _ in 0..n {
            at = (at + delta).rem_euclid(n);
            if !items[at as usize].disabled {
                active.set(Some(at as usize));
                reveal(at as usize);
                return;
            }
        }
    };

    let pick = move |opt: ComboboxOption| {
        if opt.disabled {
            return;
        }
        active.set(None);
        // 先落值再收面板：收面板会清掉草稿，显示回落成新值的标签。
        if let Some(cb) = on_change {
            cb.run(opt.value.clone());
        }
        close();
        focus_input();
    };

    let commit_custom = move |raw: &str| {
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            close();
            return;
        }
        if let Some(cb) = on_change {
            cb.run(raw);
        }
        close();
        focus_input();
    };

    let options_for_enter = options.clone();
    let on_key = move |ev: web_sys::KeyboardEvent| {
        if disabled {
            return;
        }
        if ev.is_composing() || ev.key_code() == 229 {
            return;
        }
        match ev.key().as_str() {
            "ArrowDown" | "ArrowUp" => {
                ev.prevent_default();
                let down = ev.key() == "ArrowDown";
                if !open.get_untracked() {
                    open_panel();
                }
                move_active(if down { 1 } else { -1 });
            }
            "Home" | "End" => {
                if !open.get_untracked() {
                    return;
                }
                ev.prevent_default();
                let items = filtered.get();
                let n = items.len();
                if n == 0 {
                    return;
                }
                let order: Vec<usize> = if ev.key() == "Home" {
                    (0..n).collect()
                } else {
                    (0..n).rev().collect()
                };
                // 头尾也要跳过禁用项，否则高亮会落在不可选的一行上
                for i in order {
                    if !items[i].disabled {
                        active.set(Some(i));
                        reveal(i);
                        break;
                    }
                }
            }
            "Enter" => {
                ev.prevent_default();
                if open.get_untracked() {
                    if let Some(i) = active.get_untracked() {
                        if let Some(opt) = filtered.get().get(i).cloned() {
                            pick(opt);
                            return;
                        }
                    }
                }
                let raw = text.get_untracked();
                let trimmed = raw.trim().to_string();
                if trimmed.is_empty() {
                    close();
                    return;
                }
                // 输入的文本正好等于某个候选的标签：即使列表没高亮也认它
                let exact = options_for_enter
                    .iter()
                    .find(|o| !o.disabled && o.label.trim().eq_ignore_ascii_case(&trimmed))
                    .cloned();
                if let Some(opt) = exact {
                    pick(opt);
                } else if allow_custom {
                    commit_custom(&trimmed);
                }
            }
            "Tab" => {
                // 不拦 Tab：让它照常往后走，列表只负责收起
                if open.get_untracked() {
                    close();
                }
            }
            _ => {}
        }
    };

    let aria_label = if !label.is_empty() {
        label.clone()
    } else if !placeholder.is_empty() {
        placeholder.clone()
    } else {
        "搜索".to_string()
    };
    let panel_label = if label.is_empty() {
        "候选".to_string()
    } else {
        label.clone()
    };
    let panel_id_attr = panel_id.clone();
    let panel_id_opt = panel_id.clone();
    let panel_id_active = panel_id.clone();

    view! {
        <div
            class="c-combobox"
            data-open=move || open.get().then_some("1")
            data-disabled=disabled.then_some("1")
            style=format!("width:{width}px")
            node_ref=wrap_ref
            on:focusout=move |ev: web_sys::FocusEvent| {
                if !open.get_untracked() {
                    return;
                }
                // 焦点落到包装器之外才收：点输入框、点候选都不会走到这里
                let inside = ev
                    .related_target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                    .zip(wrap_ref.get())
                    .map(|(node, wrap)| wrap.contains(Some(&node)))
                    .unwrap_or(false);
                if !inside {
                    close();
                }
            }
        >
            <input
                class="c-combobox__input"
                type="text"
                role="combobox"
                autocomplete="off"
                spellcheck="false"
                placeholder=placeholder
                aria-label=aria_label
                aria-expanded=move || open.get().to_string()
                aria-controls=panel_id_attr
                aria-autocomplete="list"
                aria-activedescendant=move || {
                    active_index().map(|i| format!("{panel_id_active}-opt-{i}"))
                }
                disabled=disabled
                // 属性给静态快照，property 给运行时；输入法组合期 text 不会变，
                // 所以不会在用户选字的中途被写回。
                value=move || text.get()
                prop:value=move || text.get()
                node_ref=input_ref
                on:pointerdown=move |_| {
                    if !open.get_untracked() {
                        open_panel();
                    }
                }
                on:input=move |ev: web_sys::Event| {
                    if disabled {
                        return;
                    }
                    let ev = ev.unchecked_ref::<web_sys::InputEvent>();
                    // 组合期的 value 只是拼音串，拿来过滤会把候选窗打乱
                    if ev.is_composing() {
                        return;
                    }
                    let raw = event_target_value(ev);
                    if !open.get_untracked() {
                        open_panel();
                    }
                    draft.set(Some(raw.clone()));
                    active.set(None);
                    if let Some(cb) = on_input {
                        cb.run(raw);
                    }
                }
                on:keydown=on_key
            />
            <span class="c-combobox__caret" aria-hidden="true">
                <Icon name=IconName::CHEVRON_DOWN size=12 />
            </span>

            <div
                class="c-combobox__panel"
                id=panel_id_opt
                role="listbox"
                aria-label=panel_label
                tabindex="-1"
                node_ref=panel_ref
                style=move || if open.get() {
                    let (l, t, w) = pos.get();
                    format!(
                        "position:fixed;left:{l}px;top:{t}px;width:{w}px;z-index:{}",
                        layer_z()
                    )
                } else {
                    "display:none".to_string()
                }
            >
                {move || {
                    let items = filtered.get();
                    if items.is_empty() {
                        return view! {
                            <div class="c-combobox__state" data-kind="empty">
                                <span>"没有匹配项"</span>
                                {allow_custom.then(|| view! {
                                    <span class="c-combobox__hint">"回车使用当前输入"</span>
                                })}
                            </div>
                        }
                            .into_any();
                    }
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, opt)| {
                            let val = opt.value.clone();
                            let label = opt.label.clone();
                            let hint = opt.hint.clone();
                            let opt_pick = opt.clone();
                            let is_disabled = opt.disabled;
                            view! {
                                <div
                                    class="c-combobox__option"
                                    role="option"
                                    id=format!("{panel_id}-opt-{i}")
                                    aria-selected=move || {
                                        (value.get().as_deref() == Some(val.as_str())).to_string()
                                    }
                                    aria-disabled=is_disabled.then_some("true")
                                    data-active=move || (active_index() == Some(i)).then_some("1")
                                    // pointerdown 而不是 click：click 之前焦点已经离开输入框，
                                    // 会被自己的「点外」判定先收一次列表。
                                    on:pointerdown=move |ev| {
                                        ev.prevent_default();
                                        if is_disabled {
                                            return;
                                        }
                                        pick(opt_pick.clone());
                                    }
                                    on:pointerenter=move |_| {
                                        if !is_disabled {
                                            active.set(Some(i));
                                        }
                                    }
                                >
                                    <span class="c-combobox__label">{label}</span>
                                    {(!hint.is_empty()).then(|| view! {
                                        <span class="c-combobox__hint">{hint}</span>
                                    })}
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
        </div>
    }
}
