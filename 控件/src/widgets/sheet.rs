use leptos::html;
use leptos::prelude::*;

use crate::base::i18n::t;
use crate::base::icon::{Icon, IconName};
use crate::base::overlay::{ExitGate, OverlayHost, OverlayId, OverlayKind, ParentScope};

/// 流程面板宽度档。`Md` 正好命中玻璃档 `sheet-520x360`（宽 520、高 360）。
///
/// 为什么不是任意宽度：贴图必须与元素逐像素一致（`feImage` 不按框拉伸），
/// 所以只有 `Md` 加 `data-glass`；另外两档走 CSS 涂层，不套折射。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SheetSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl SheetSize {
    fn width(self) -> u32 {
        match self {
            Self::Sm => 400,
            Self::Md => 520,
            Self::Lg => 680,
        }
    }

    fn token(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }

    /// 只有尺寸完全对得上的档位才贴玻璃贴图；其它档加了会被硬切。
    fn glass(self) -> Option<&'static str> {
        match self {
            Self::Md => Some("sheet-520x360"),
            _ => None,
        }
    }
}

/// 流程面板：标题栏 + 可滚动身体 + 固定动作区。
///
/// 和 Dialog 的差别不在尺寸，而在归属：Dialog 是一次问答，Sheet 是某个窗口的
/// 流程层（能一直看见自己在改什么，动作区永远贴在底边）。Esc / 遮罩 / inert /
/// Tab 循环 / 焦点归还全部交给 OverlayHost。
#[component]
pub fn Sheet(
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] description: String,
    #[prop(optional)] size: SheetSize,
    #[prop(default = true)] dismissible: bool,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] footer: Option<Children>,
) -> impl IntoView {
    let sheet_ref = NodeRef::<html::Div>::new();
    let host = OverlayHost::use_or_provide();
    let layer = RwSignal::new(None::<OverlayId>);
    let level = RwSignal::new(1usize);
    // 退场态：`open` 已 false，但面板还挂在屏幕上播过渡、也还留在栈上。
    let closing = RwSignal::new(false);
    let gate = ExitGate::new();
    let is_open = move || open.get().unwrap_or(false);
    let width = size.width();

    let close = move || {
        // 幂等：退场态里再按 Esc / 再点遮罩（此时本层仍是栈顶）不能再叫一次关闭。
        if !is_open() {
            return;
        }
        if let Some(cb) = on_close {
            cb.run(());
        }
    };

    // 入栈/出栈跟着 open 走。z、Esc、inert、焦点全部由 OverlayHost 决定。
    Effect::new(move |_| {
        if is_open() {
            // 退场期间又打开：作废那张票，面板原地回到 open 态。
            gate.cancel();
            closing.set(false);
            if layer.get_untracked().is_none() {
                let dismiss = if dismissible {
                    Callback::new(move |_| close())
                } else {
                    // 不可关闭的流程：仍然入栈挡住下层，Esc 被吞掉但不关。
                    Callback::new(|_| {})
                };
                let id = host.push(OverlayKind::Sheet, dismiss);
                level.set(host.depth());
                layer.set(Some(id));
            }
            if let Some(el) = sheet_ref.get() {
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
    });

    on_cleanup({
        let host = host;
        move || {
            if let Some(id) = layer.get_untracked() {
                host.remove(id);
            }
        }
    });

    let has_title = !title.is_empty();
    let has_desc = !description.is_empty();
    // 有标题就用标题当可访问名；没有时给个兜底，别让读屏念出空名字。
    // 兜底查文案表，且放在闭包里 —— 切语言时这个属性要跟着变。
    let title_for_label = title.clone();
    let sheet_label = move || {
        if has_title {
            title_for_label.clone()
        } else {
            t().sheet_label.to_string()
        }
    };

    view! {
        <div
            class="c-sheet-scrim"
            // 「在场上」判栈句柄而不是 open：关闭后遮罩与面板要留 EXIT_MS 播退场。
            style=move || if layer.get().is_some() {
                let z = layer
                    .get()
                    .map(|id| host.z(id))
                    .unwrap_or(OverlayKind::Sheet.base_z());
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
                // 只有点到遮罩本身才算「点外」：面板内部的 pointerdown 会冒泡上来。
                if ev.target() == ev.current_target() {
                    close();
                }
            }
        >
            <div
                class="c-sheet c-glass-sheet"
                role="dialog"
                aria-modal="true"
                aria-label=sheet_label
                tabindex="-1"
                data-size=size.token()
                data-glass=size.glass()
                style=format!("--sheet-w:{width}px")
                node_ref=sheet_ref
            >
                <div class="c-sheet__head">
                    {has_title.then(|| view! { <h2 class="c-sheet__title">{title.clone()}</h2> })}
                    {dismissible.then(|| view! {
                        <button
                            class="c-sheet__close"
                            type="button"
                            aria-label=move || t().close.to_string()
                            on:click=move |_| close()
                        >
                            <span class="c-sheet__close-icon">
                                <Icon name=IconName::X size=13 />
                            </span>
                        </button>
                    })}
                    {has_desc.then(|| view! { <p class="c-sheet__desc">{description.clone()}</p> })}
                </div>
                // 面板里放表单 / 下拉是常态：动作区与身体都归这一层管，
                // 里面再开下拉时新层挂在本面板下面，不会反过来把面板请出。
                <ParentScope id=layer>
                    <div class="c-sheet__body">{children.map(|c| c())}</div>
                    {footer.map(|f| view! { <div class="c-sheet__foot">{f()}</div> })}
                </ParentScope>
            </div>
        </div>
    }
}
