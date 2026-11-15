use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};
use crate::base::id::next_id;

/// 按分隔符切开一段文本。中英文标点都算 —— 输入法上屏的「，」不会走 keydown，
/// 只能在值里认，所以这里是唯一的真相。
fn split_tokens(raw: &str) -> Vec<String> {
    raw.split(|c: char| matches!(c, ',' | ';' | '，' | '；' | '\n' | '\r'))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// 令牌输入：一个会换行的输入框，已确认的 token 在左，正在敲的文字在右。
///
/// 受控点是 `Vec<String>`（token 本身），输入框里的草稿不受控 —— 草稿交给 DOM
/// 保管，中文输入法才不会被每次 `input` 的声明式写回打断。这也是为什么输入框
/// 是与 token 并列的静态节点，而不是被包在每次重建的动态块里：换行重排时焦点
/// 不会掉到 body 上。
#[component]
pub fn TokenField(
    #[prop(into, optional)] tokens: MaybeProp<Vec<String>>,
    #[prop(into, optional)] placeholder: String,
    #[prop(into, optional)] label: String,
    /// 0 表示不限
    #[prop(optional)] max: usize,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<Vec<String>>>,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();
    let root_ref = NodeRef::<html::Div>::new();
    let count_id = next_id("gd-token-count");
    let count_id_attr = count_id.clone();

    let items = move || tokens.get().unwrap_or_default();
    let full = move || max > 0 && items().len() >= max;

    let focus_input = move || {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    // 一次调用只发一次 on_change：粘贴进来的一整段要合并成一次提交，
    // 否则订阅方会收到 N 次中间态。
    let add = move |raw: &str| {
        let mut list = items();
        let before = list.len();
        for part in split_tokens(raw) {
            if max > 0 && list.len() >= max {
                break;
            }
            // 重复值不添加：同一个标签敲两遍不应该出现两枚
            if list.iter().any(|t| t == &part) {
                continue;
            }
            list.push(part);
        }
        if list.len() != before {
            if let Some(cb) = on_change {
                cb.run(list);
            }
        }
    };

    let remove_at = move |index: usize| {
        let mut list = items();
        if index >= list.len() {
            return;
        }
        list.remove(index);
        if let Some(cb) = on_change {
            cb.run(list);
        }
    };

    let clear_input = move || {
        if let Some(el) = input_ref.get() {
            el.set_value("");
        }
    };

    let commit = move || {
        let Some(el) = input_ref.get() else { return };
        let raw = el.value();
        if raw.trim().is_empty() || full() {
            return;
        }
        add(&raw);
        el.set_value("");
    };

    let dft = if placeholder.is_empty() {
        "输入后回车".to_string()
    } else {
        placeholder.clone()
    };

    view! {
        <div
            class="c-tokenfield"
            data-disabled=disabled.then_some("1")
            data-full=move || full().then_some("1")
            node_ref=root_ref
            // 点在 token 之间的缝里也应该进入输入，否则 cursor:text 是在骗人
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                if disabled {
                    return;
                }
                let target = ev.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok());
                let Some(root) = root_ref.get() else { return };
                let on_blank = target
                    .map(|t| t.is_same_node(Some(root.unchecked_ref::<web_sys::Node>())))
                    .unwrap_or(false);
                if on_blank {
                    ev.prevent_default();
                    focus_input();
                }
            }
        >
            // 动态块只包 token：重建时输入框是它的兄弟节点，不受影响，焦点不丢。
            {move || {
                items()
                    .iter()
                    .enumerate()
                    .map(|(i, token)| {
                        let text = token.clone();
                        view! {
                            <span class="c-tokenfield__token" data-value=text.clone()>
                                <span class="c-tokenfield__label">{text.clone()}</span>
                                <button
                                    class="c-tokenfield__remove"
                                    type="button"
                                    // 删除按钮不进 Tab 序列：token 不是焦点目标，
                                    // 键盘删除走输入框里的 Backspace
                                    tabindex="-1"
                                    aria-label=format!("移除 {text}")
                                    disabled=disabled
                                    // 按下时别把焦点从输入框带走：删完还要接着打字
                                    on:pointerdown=move |ev: web_sys::PointerEvent| {
                                        ev.prevent_default();
                                    }
                                    on:click=move |_| {
                                        if disabled {
                                            return;
                                        }
                                        remove_at(i);
                                        focus_input();
                                    }
                                >
                                    <Icon name=IconName::X size=10 />
                                </button>
                            </span>
                        }
                    })
                    .collect_view()
            }}
            <input
                class="c-tokenfield__input"
                type="text"
                autocomplete="off"
                spellcheck="false"
                aria-label=label
                placeholder=move || if full() {
                    "已达上限".to_string()
                } else {
                    dft.clone()
                }
                aria-describedby=(max > 0).then(|| count_id_attr)
                disabled=disabled
                readonly=move || full()
                node_ref=input_ref
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if disabled || ev.is_composing() || ev.key_code() == 229 {
                        return;
                    }
                    let has_text = input_ref
                        .get()
                        .map(|el| !el.value().trim().is_empty())
                        .unwrap_or(false);
                    match ev.key().as_str() {
                        "Enter" => {
                            // 空输入框里的 Enter 留给外层表单提交，不在这里吞掉
                            if has_text {
                                ev.prevent_default();
                                commit();
                            }
                        }
                        "," | ";" | "，" | "；" => {
                            ev.prevent_default();
                            commit();
                        }
                        "Backspace" => {
                            // 输入框空时才删 token，否则删的是正在敲的字
                            if !has_text {
                                ev.prevent_default();
                                let n = items().len();
                                if n > 0 {
                                    remove_at(n - 1);
                                }
                            }
                        }
                        "Escape" => {
                            if has_text {
                                ev.prevent_default();
                                clear_input();
                            }
                        }
                        _ => {}
                    }
                }
                on:input=move |ev: web_sys::Event| {
                    if disabled {
                        return;
                    }
                    // 输入法上屏的分隔符只出现在值里，keydown 拦不到
                    let raw = event_target_value(&ev);
                    let has_sep = raw
                        .contains(|c: char| matches!(c, ',' | ';' | '，' | '；' | '\n'));
                    if has_sep {
                        add(&raw);
                        clear_input();
                    }
                }
                on:paste=move |ev: web_sys::ClipboardEvent| {
                    let Some(data) = ev.clipboard_data() else { return };
                    let Ok(text) = data.get_data("text/plain") else { return };
                    // 不带分隔符的单段文本照常粘贴，别抢用户的普通操作
                    let batch = text.contains(|c: char| {
                        matches!(c, ',' | ';' | '，' | '；' | '\n' | '\r')
                    });
                    if !batch {
                        return;
                    }
                    ev.prevent_default();
                    add(&text);
                }
            />
            <span class="c-tokenfield__count" id=count_id hidden=move || max == 0>
                {move || format!("{}/{}", items().len(), max)}
            </span>
        </div>
    }
}
