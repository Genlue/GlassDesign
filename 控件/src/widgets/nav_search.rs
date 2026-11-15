//! 导航栏搜索入口 NavSearch。
//!
//! 为什么不直接包一层 `SearchField`：`SearchField` 把宽度写成内联 `style="width:Npx"`，
//! 而 08-navigation.css 的 `.c-navsearch .c-field { width: 100% }` 是样式表规则 ——
//! 内联永远赢，塞进导航栏就只能是 180px 的定宽框。这里复用同一套 DOM 契约
//! （`.c-field` / `__icon` / `__input` + `c-iconbtn` 清除钮），但不写内联宽度。
//!
//! 全局快捷键挂在 window 上，所以必须跟着 owner 一起释放：
//! 画廊切分区会重建组件，不摘就会一按 ⌘K 同时聚焦好几个已经不在文档里的输入框。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::icon::{Icon, IconName};

/// 全局热键的持有者：只为了在 owner 释放时把 window 监听摘掉。
///
/// 为什么不用 `on_cleanup`：它要求闭包 `Send + Sync`，而 `Closure` 明确不是。
/// 走 `StoredValue<_, LocalStorage>` 的 Drop 是仓库里已有的解法（见 overlay.rs）。
struct Hotkey {
    win: web_sys::Window,
    listener: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        let _ = self
            .win
            .remove_event_listener_with_callback("keydown", self.listener.as_ref().unchecked_ref());
    }
}

/// 导航栏搜索。
///
/// - `shortcut`：快捷键串（"⌘K" / "Ctrl+K"）。它决定绑哪个字母键（取最后一个
///   字母数字字符），并原样进 `aria-keyshortcuts`；修饰键则 Ctrl 与 Cmd 都接受
///   （同一套代码要能跑在 Windows 与 macOS 上，不做平台分支）。
/// - `on_submit`：回车提交当前值；Esc 清空并失焦（只清空不失焦的话，
///   用户按完 Esc 还停在框里，下一次输入会接着上一次的词）。
#[component]
pub fn NavSearch(
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(optional)] on_submit: Option<Callback<String>>,
    #[prop(into, optional)] shortcut: String,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();
    let placeholder = if placeholder.is_empty() {
        "搜索".to_string()
    } else {
        placeholder
    };
    let shortcut = if shortcut.is_empty() {
        "Ctrl+K".to_string()
    } else {
        shortcut
    };
    // 从 "⌘K" / "Ctrl+K" 里取最后一个字母数字字符当热键；取不到就退回 K。
    let hotkey = shortcut
        .chars()
        .rev()
        .find(|c| c.is_alphanumeric())
        .unwrap_or('K');
    // aria-keyshortcuts 要的是规范写法（Control+K），不是给人看的 ⌘K
    let aria_keys = format!("Control+{}", hotkey.to_ascii_uppercase());
    let has_value = move || !value.get().unwrap_or_default().is_empty();
    // 热键字符预转小写：监听器每次按键都要比一次，别在回调里分配字符串。
    let hotkey_lc = hotkey.to_ascii_lowercase().to_string();

    if let Some(win) = web_sys::window() {
        let listener = Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if ev.is_composing() {
                return;
            }
            if !(ev.ctrl_key() || ev.meta_key()) || ev.alt_key() {
                return;
            }
            if ev.key().to_ascii_lowercase() != hotkey_lc {
                return;
            }
            ev.prevent_default();
            if let Some(el) = input_ref.get_untracked() {
                let _ = el.focus();
                // 已有内容时选中全部：接着输入就是替换，而不是追加
                let _ = el.select();
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        let _ = win.add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref());
        // 句柄自己负责摘监听：`on_cleanup` 的闭包要求 Send + Sync，而 Closure 不是。
        // 放进 LocalStorage 的 StoredValue，owner 释放时 Drop 会跑（和 OverlayHost 同一招）。
        let _hotkey = StoredValue::new_local(Hotkey { win, listener });
    }

    let emit = move |v: String| {
        if let Some(cb) = on_input {
            cb.run(v);
        }
    };

    view! {
        <div class="c-navsearch">
            // 不写 .c-search：那个类带 180→240px 的定宽过渡，导航栏里要的是满宽
            <div class="c-field">
                <span class="c-field__icon"><Icon name=IconName::SEARCH size=14 /></span>
                <input
                    class="c-field__input selectable"
                    type="text"
                    placeholder=placeholder.clone()
                    aria-label=placeholder
                    aria-keyshortcuts=aria_keys
                    value=move || value.get().unwrap_or_default()
                    prop:value=move || value.get().unwrap_or_default()
                    node_ref=input_ref
                    on:input=move |ev| {
                        if let Some(cb) = on_input {
                            cb.run(event_target_value(&ev));
                        }
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        match ev.key().as_str() {
                            // Esc：清空 + 失焦（不是「清空后重新聚焦」）
                            "Escape" => {
                                ev.prevent_default();
                                if let Some(cb) = on_input {
                                    cb.run(String::new());
                                }
                                if let Some(el) = input_ref.get_untracked() {
                                    let _ = el.blur();
                                }
                            }
                            "Enter" => {
                                ev.prevent_default();
                                if let Some(cb) = on_submit {
                                    cb.run(value.get_untracked().unwrap_or_default());
                                }
                            }
                            _ => {}
                        }
                    }
                />
                <Show when=has_value>
                    <button
                        class="c-iconbtn c-iconbtn--sm"
                        type="button"
                        aria-label="清除"
                        on:click=move |_| {
                            emit(String::new());
                            if let Some(el) = input_ref.get_untracked() {
                                let _ = el.focus();
                            }
                        }
                    >
                        <Icon name=IconName::X_CIRCLE size=14 />
                    </button>
                </Show>
            </div>
        </div>
    }
}
