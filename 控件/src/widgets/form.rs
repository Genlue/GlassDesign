use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::widgets::inline_message::{InlineMessage, Tone};

/// 表单上下文：只传「整体状态」，不登记字段。
///
/// 字段的错误由 `FormField` 自己显示，`Form` 不知道有哪些字段 —— 一旦引入
/// 注册表，字段就得向表单登记、注销、上报值，这套协议比控件本身还重。
/// 这里只把 `disabled / loading / on_cancel` 放进 context，让 `FormActions`
/// 不必由调用方手工同步一份。
#[derive(Clone, Copy)]
struct FormCtx {
    disabled: MaybeProp<bool>,
    loading: MaybeProp<bool>,
    on_cancel: Option<Callback<()>>,
}

/// 表单编排：Enter / 提交按钮走同一条路，整体禁用与提交中由 `aria-busy` 表达。
///
/// 表单级错误汇总排在动作行**之上**：字段错误在字段下方，整体错误在提交处，
/// 两者不在同一个位置，用户才不会以为是某一格填错了。
#[component]
pub fn Form(
    #[prop(optional)] on_submit: Option<Callback<()>>,
    #[prop(optional)] on_cancel: Option<Callback<()>>,
    #[prop(into, optional)] disabled: MaybeProp<bool>,
    #[prop(into, optional)] loading: MaybeProp<bool>,
    #[prop(into, optional)] error: MaybeProp<String>,
    #[prop(into, optional)] label: String,
    children: Children,
    #[prop(optional)] actions: Option<Children>,
) -> impl IntoView {
    provide_context(FormCtx {
        disabled,
        loading,
        on_cancel,
    });

    let is_disabled = move || disabled.get().unwrap_or(false);
    let is_loading = move || loading.get().unwrap_or(false);
    let submit = move || {
        if is_disabled() || is_loading() {
            return;
        }
        if let Some(cb) = on_submit {
            cb.run(());
        }
    };

    view! {
        <form
            class="c-form"
            aria-label=label
            aria-busy=move || is_loading().then_some("true")
            aria-disabled=move || is_disabled().then_some("true")
            data-disabled=move || is_disabled().then_some("1")
            // 不用 action：CSR 里没有后端，提交一律由调用方接管
            on:submit=move |ev| {
                ev.prevent_default();
                submit();
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() != "Enter" || ev.shift_key() || ev.is_composing() {
                    return;
                }
                let tag = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|el| el.tag_name())
                    .unwrap_or_default();
                // 按钮自己会发 submit；文本域的 Enter 是换行，不该被当成提交
                if tag.eq_ignore_ascii_case("BUTTON") || tag.eq_ignore_ascii_case("TEXTAREA") {
                    return;
                }
                ev.prevent_default();
                submit();
            }
        >
            // 汇总区排在字段之前：`FormActions` 既可以走 actions 槽、也可以直接写在
            // children 里（后者更常用），只有放在最前面才保证「永远在动作行之上」。
            {move || {
                let msg = error.get().unwrap_or_default();
                (!msg.trim().is_empty()).then(|| view! {
                    <InlineMessage tone=Tone::Danger description=msg />
                })
            }}
            {children()}
            {actions.map(|a| a())}
        </form>
    }
}

/// 表单动作行：左取消右提交。
///
/// 提交按钮必须是 `type="submit"`，所以这里不套 `Button`（它固定 `type="button"`），
/// 只复用 `.c-btn` / `.c-btn--primary` 的材质类。
#[component]
pub fn FormActions(
    #[prop(into, optional)] submit_label: String,
    #[prop(into, optional)] cancel_label: String,
    #[prop(optional)] on_cancel: Option<Callback<()>>,
    /// 不套在 `Form` 里时自己声明禁用态
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    let ctx = use_context::<FormCtx>();
    let busy = move || {
        ctx.map(|c| c.loading.get().unwrap_or(false))
            .unwrap_or(false)
    };
    let off = move || {
        disabled
            || ctx
                .map(|c| c.disabled.get().unwrap_or(false))
                .unwrap_or(false)
    };
    let cancel_cb = on_cancel.or_else(|| ctx.and_then(|c| c.on_cancel));
    let submit_text = if submit_label.is_empty() {
        "提交".to_string()
    } else {
        submit_label
    };
    let cancel_text = if cancel_label.is_empty() {
        "取消".to_string()
    } else {
        cancel_label
    };

    view! {
        <div class="c-form__actions">
            <button
                class="c-btn"
                type="button"
                disabled=move || off() || busy()
                on:click=move |_| {
                    if let Some(cb) = cancel_cb {
                        cb.run(());
                    }
                }
            >
                <span class="c-btn__label">{cancel_text}</span>
            </button>
            <button
                class="c-btn c-btn--primary"
                type="submit"
                aria-busy=move || busy().then_some("true")
                disabled=move || off() || busy()
            >
                <span class="c-btn__label">
                    {move || if busy() { "提交中…".to_string() } else { submit_text.clone() }}
                </span>
            </button>
        </div>
    }
}
