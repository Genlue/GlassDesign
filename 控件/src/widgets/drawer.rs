use leptos::html;
use leptos::prelude::*;

use crate::base::i18n::t;
use crate::base::overlay::{ExitGate, OverlayHost, OverlayId, OverlayKind};

/// 抽屉从哪条边拉出。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DrawerSide {
    Left,
    #[default]
    Right,
    Bottom,
}

impl DrawerSide {
    fn token(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Bottom => "bottom",
        }
    }
}

/// 拖动关闭的位移阈值（像素）。低于它的拖动视为「拉了一半又反悔」，
/// 松手弹回原位 —— 触控板上一次轻蹭就关掉面板是很烦的。
const DRAG_CLOSE_PX: f64 = 80.0;
/// 甩动速度阈值（px/ms）：快速下扫即使位移不够也认作关闭。
const DRAG_FLICK: f64 = 0.5;

/// 贴边抽屉（模态）。`width` 是「厚度」：左右抽屉是宽度，底部抽屉是高度。
///
/// 只有 Bottom 渲染抓握条：左右抽屉的抓握条在 CSS 里是竖着的 22px 细边，
/// 真要做拖拽得同时处理「拖宽」与「拖关」两套手势，超出本批范围（见报告）。
#[component]
pub fn Drawer(
    #[prop(into, optional)] open: MaybeProp<bool>,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] side: DrawerSide,
    #[prop(optional)] width: u32,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let sheet_ref = NodeRef::<html::Div>::new();
    let grip_ref = NodeRef::<html::Button>::new();
    let host = OverlayHost::use_or_provide();
    let layer = RwSignal::new(None::<OverlayId>);
    // 退场态：`open` 已 false，但抽屉还挂在屏幕上播过渡、也还留在栈上。
    let closing = RwSignal::new(false);
    let gate = ExitGate::new();
    // 拖动进度：1 = 完全展开，0 = 完全收起。CSS 用它做位移与遮罩浓度。
    let progress = RwSignal::new(1.0f64);
    let dragging = RwSignal::new(false);
    let drag = StoredValue::new_local(None::<(f64, f64, f64, f64, i32)>);
    let is_open = move || open.get().unwrap_or(false);
    let width = if width == 0 { 320 } else { width };

    let close = move || {
        // 幂等：退场态里再按 Esc / 再点遮罩（此时本层仍是栈顶）不能再叫一次关闭。
        if !is_open() {
            return;
        }
        if let Some(cb) = on_close {
            cb.run(());
        }
    };

    Effect::new(move |_| {
        if is_open() {
            // 退场期间又打开：作废那张票，抽屉原地回到展开态（进度直接回 1）。
            gate.cancel();
            closing.set(false);
            progress.set(1.0);
            if layer.get_untracked().is_none() {
                let id = host.push(OverlayKind::Drawer, Callback::new(move |_| close()));
                layer.set(Some(id));
            }
            if let Some(el) = sheet_ref.get() {
                if let Some(id) = layer.get_untracked() {
                    host.bind(id, el.into());
                }
            }
        } else if let Some(id) = layer.get_untracked() {
            drag.set_value(None);
            dragging.set(false);
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

    // —— 底部抽屉的抓握条：按下记录起点，移动改进度，松手按位移/速度决定去向 ——
    // 元组是 (起点 y, 上一次 y, 当前 y, 上一次时间, pointerId)。
    // 「上一次 y」单独存是为了算甩动速度：拿总位移除以最后一帧的时长，
    // 会把慢慢拖了 60px 也判成 3px/ms 的快甩。
    let on_down = move |ev: web_sys::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        if let Some(grip) = grip_ref.get() {
            let _ = grip.set_pointer_capture(ev.pointer_id());
        }
        let y = ev.client_y() as f64;
        drag.set_value(Some((y, y, y, js_sys::Date::now(), ev.pointer_id())));
        dragging.set(true);
        ev.prevent_default();
    };

    let on_move = move |ev: web_sys::PointerEvent| {
        let Some((start, _, last, _, id)) = drag.get_value() else { return };
        if ev.pointer_id() != id {
            return;
        }
        let Some(sheet) = sheet_ref.get() else { return };
        let height = sheet.offset_height() as f64;
        if height <= 0.0 {
            return;
        }
        let y = ev.client_y() as f64;
        let dy = y - start;
        let next = (1.0 - dy / height).clamp(0.0, 1.0);
        progress.set(next);
        drag.set_value(Some((start, last, y, js_sys::Date::now(), id)));
    };

    let finish = move |cancelled: bool| {
        let Some((start, prev, last, last_t, _)) = drag.get_value() else { return };
        drag.set_value(None);
        dragging.set(false);
        if cancelled {
            progress.set(1.0);
            return;
        }
        let dy = last - start;
        let dt = (js_sys::Date::now() - last_t).max(1.0);
        let flick = (last - prev) / dt > DRAG_FLICK;
        if dy >= DRAG_CLOSE_PX || flick {
            close();
        } else {
            progress.set(1.0);
        }
    };

    let show_grip = side == DrawerSide::Bottom;

    view! {
        <div
            class="c-drawer c-drawer--fixed"
            data-side=side.token()
            // `data-open` 跟着 open 走（不是退场态）：退场一开始遮罩就不再吃点击，
            // 这 140ms 里点背景不会被一层正在淡出的遮罩吞掉。
            data-open=move || is_open().then_some("1")
            data-dragging=move || dragging.get().then_some("1")
            // 退场态给 CSS 用；收起时刻看栈句柄（下一段 style）。
            data-state=move || {
                if closing.get() {
                    Some("closing")
                } else if layer.get().is_some() {
                    Some("open")
                } else {
                    None
                }
            }
            style=move || if layer.get().is_some() {
                let z = layer
                    .get()
                    .map(|id| host.z(id))
                    .unwrap_or(OverlayKind::Drawer.base_z());
                format!("--dw-size:{width}px;--dw-p:{};z-index:{z}", progress.get())
            } else {
                "display:none".to_string()
            }
        >
            <div
                class="c-drawer__scrim"
                on:pointerdown=move |ev| {
                    // 只有点到遮罩本身才算「点外」：sheet 是它的子节点。
                    if ev.target() == ev.current_target() {
                        close();
                    }
                }
            >
                <div
                    class="c-drawer__sheet c-glass-sheet"
                    role="dialog"
                    aria-modal="true"
                    aria-label=move || t().drawer_label.to_string()
                    tabindex="-1"
                    node_ref=sheet_ref
                >
                    {show_grip.then(|| view! {
                        <button
                            class="c-drawer__grip"
                            type="button"
                            aria-label=move || t().drag_close.to_string()
                            node_ref=grip_ref
                            on:pointerdown=on_down
                            on:pointermove=on_move
                            on:pointerup=move |_| finish(false)
                            on:pointercancel=move |_| finish(true)
                        >
                            <i class="c-drawer__bar"></i>
                        </button>
                    })}
                    <div class="c-drawer__body">{children.map(|c| c())}</div>
                </div>
            </div>
        </div>
    }
}
