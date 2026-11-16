use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::base::overlay::{ExitGate, OverlayHost, OverlayId, OverlayKind};

#[component]
pub fn Dialog(
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] description: String,
    #[prop(optional)] width: u32,
    #[prop(default = true)] dismissible: bool,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] actions: Option<Children>,
) -> impl IntoView {
    let dialog_ref = NodeRef::<html::Div>::new();
    let host = OverlayHost::use_or_provide();
    let layer = RwSignal::new(None::<OverlayId>);
    // 二级起遮罩要更浅：由栈深决定，不再写死 data-level="1"。
    let level = RwSignal::new(1usize);
    // 退场态：`open` 已 false，但面板还挂在屏幕上播过渡、也还留在栈上。
    // 出栈（→ inert 解除 + 焦点归还）因此也晚 EXIT_MS —— 遮罩正在淡出，
    // 这 140ms 内点到背景会丢一次点击，是有意换来的「不啪一下消失」。
    let closing = RwSignal::new(false);
    let gate = ExitGate::new();
    let is_open = move || open.get().unwrap_or(false);
    let width = if width == 0 { 300 } else { width };

    let close = move || {
        // 幂等：退场态里再按 Esc / 再点遮罩（此时本层仍是栈顶）不能再叫一次关闭。
        if !is_open() {
            return;
        }
        if let Some(cb) = on_close {
            cb.run(());
        }
    };

    // 入栈/出栈跟着 open 走。z、Esc、inert、焦点全部交给 OverlayHost。
    Effect::new({
        let host = host.clone();
        move |_| {
            if is_open() {
                // 退场期间又打开：作废那张票，面板原地回到 open 态（inert 与焦点都没动过）。
                gate.cancel();
                closing.set(false);
                if layer.get_untracked().is_none() {
                    let dismiss = if dismissible {
                        Callback::new(move |_| close())
                    } else {
                        // 不可关闭的模态：仍然入栈（挡住下层），Esc 被吞掉但不关。
                        Callback::new(|_| {})
                    };
                    let id = host.push(OverlayKind::Dialog, dismiss);
                    level.set(host.depth());
                    layer.set(Some(id));
                }
                if let Some(el) = dialog_ref.get() {
                    if let Some(id) = layer.get_untracked() {
                        host.bind(id, el.into());
                    }
                }
            } else if let Some(id) = layer.get_untracked() {
                if closing.get_untracked() {
                    // 已经在退场，绝不能再挂第二个出栈定时器。
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
        }
    });

    on_cleanup({
        let host = host.clone();
        move || {
            if let Some(id) = layer.get_untracked() {
                host.remove(id);
            }
        }
    });

    let has_head = !title.is_empty() || !description.is_empty();

    view! {
        <div
                    class="c-dialog-scrim"
                    // 「在场上」判栈句柄而不是 open：关闭后遮罩与面板要留 EXIT_MS 播退场。
                    style=move || if layer.get().is_some() {
                        let z = layer.get().map(|id| host.z(id)).unwrap_or(OverlayKind::Dialog.base_z());
                        format!("z-index:{z}")
                    } else {
                        "display:none".to_string()
                    }
                    data-level=move || level.get().to_string()
                    data-state=move || {
                        if closing.get() {
                            Some("closing")
                        } else if layer.get().is_some() {
                            Some("open")
                        } else {
                            None
                        }
                    }
                    on:pointerdown=move |ev| {
                        if !dismissible {
                            return;
                        }
                        if ev.target() == ev.current_target() {
                            close();
                        }
                    }
                >
                    <div
                        class="c-dialog c-glass-sheet"
                        role="dialog"
                        aria-modal="true"
                        aria-label=title.clone()
                        data-level="1"
                        style=format!("--dlg-w:{width}px")
                        tabindex="-1"
                        node_ref=dialog_ref
                    >
                        {has_head.then(|| view! {
                            <div class="c-dialog__head">
                                <span class="c-dialog__icon"><Icon name=IconName::INFO size=17 /></span>
                                <div class="l-fill">
                                    {(!title.is_empty()).then(|| view! { <div class="c-dialog__title">{title.clone()}</div> })}
                                    {(!description.is_empty()).then(|| view! { <div class="c-dialog__desc">{description.clone()}</div> })}
                                </div>
                            </div>
                        })}
                        {children.map(|c| view! { <div class="c-dialog__body">{c()}</div> })}
                        {actions.map(|a| view! { <div class="c-dialog__actions">{a()}</div> })}
                    </div>
        </div>
    }
}
