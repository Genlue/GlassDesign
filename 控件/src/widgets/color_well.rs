use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::overlay::{OverlayHost, OverlayId, OverlayKind};
use crate::widgets::color_picker::{hex_to_rgb, normalize_hex, ColorPicker, SwatchItem};

/* ==========================================================================
   颜色井 ColorWell

   井是控件，面板是浮层：面板走 `OverlayHost`（z 与 Esc 的唯一权威）+ 自己
   算 `position:fixed` 的落点（照抄 Select/DatePicker 的做法，避开卡片的
   `overflow` 裁剪）。非模态，所以不 `bind`，也不做焦点陷阱。
   ========================================================================== */

#[component]
pub fn ColorWell(
    #[prop(into, optional)] value: MaybeProp<String>,
    /// 无障碍名；`show_label` 打开时同时也是可见标签
    #[prop(into, optional)] label: String,
    #[prop(optional)] show_label: bool,
    /// 井右侧显示十六进制文本
    #[prop(optional)] show_hex: bool,
    #[prop(optional)] disabled: bool,
    /// 面板里的预设；留空用内置的 8 个强调色
    #[prop(into, optional)] presets: Vec<SwatchItem>,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    let host = OverlayHost::use_or_provide();
    let trigger_ref = NodeRef::<html::Button>::new();
    let panel_ref = NodeRef::<html::Div>::new();

    let open = RwSignal::new(false);
    let layer = RwSignal::new(None::<OverlayId>);
    let pos = RwSignal::new((0.0f64, 0.0f64));
    // 调用方不回写 value 时（半受控用法），井也要跟着面板动，否则面板里调色
    // 而井纹丝不动，看起来像没生效。有 value 时以 value 为准。
    let local = RwSignal::new(None::<String>);

    let current = move || {
        let raw = value.get().or_else(|| local.get()).unwrap_or_default();
        if raw.trim().is_empty() {
            String::new()
        } else {
            normalize_hex(&raw)
        }
    };
    let fill_css = move || {
        let raw = value.get().or_else(|| local.get()).unwrap_or_default();
        let Some((r, g, b)) = hex_to_rgb(&raw) else {
            return "transparent".to_string();
        };
        format!("rgb({r} {g} {b})")
    };

    let name = if label.is_empty() {
        "颜色".to_string()
    } else {
        label.clone()
    };
    let aria = {
        let name = name.clone();
        move || well_label(&name, &current())
    };
    let title = move || well_label(&name, &current());
    let dialog_label = if label.is_empty() {
        "颜色取色器".to_string()
    } else {
        format!("{label}取色器")
    };

    // 关闭只有一条路：出栈 → 收起 → 回焦触发器。点外、Esc（host 回调）、
    // 再点一次井都走这里。
    let close = move || {
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
        open.set(false);
        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    };
    let dismiss = Callback::new(move |_| close());

    let open_panel = move || {
        if disabled || layer.get_untracked().is_some() {
            return;
        }
        if let Some(el) = trigger_ref.get() {
            let r = el.get_bounding_client_rect();
            pos.set((r.left(), r.bottom() + 6.0));
        }
        let id = host.push(OverlayKind::Popover, dismiss);
        layer.set(Some(id));
        open.set(true);
        // 焦点移进面板：非模态不禁背景，但「打开后键盘直接落在面板里」
        // 是浮层的常规预期；必须等一帧，此时面板还带着 display:none。
        let target = panel_ref;
        schedule(move || {
            let Some(panel) = target.get() else { return };
            if let Ok(Some(el)) = panel.query_selector(
                "[tabindex='0'], button:not([disabled]), input:not([disabled])",
            ) {
                if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
        });
    };

    let toggle = move || {
        if open.get_untracked() {
            close();
        } else {
            open_panel();
        }
    };

    let pick = move |hex: String| {
        local.set(Some(hex.clone()));
        if let Some(cb) = on_change {
            cb.run(hex);
        }
    };

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

    view! {
        // 外层只是「井 + 标签 + 十六进制」的一行，用既有布局工具类，不新造样式
        <div class="l-row l-row--3" style="min-width:0">
            <button
                class="c-colorwell"
                type="button"
                aria-haspopup="dialog"
                aria-expanded=move || open.get().to_string()
                aria-label=aria
                title=title
                aria-disabled=disabled.then_some("true")
                data-disabled=disabled.then_some("1")
                disabled=disabled
                node_ref=trigger_ref
                style=move || format!("--cw-color:{}", fill_css())
                on:click=move |_| toggle()
            >
                <span class="c-colorwell__fill" aria-hidden="true"></span>
            </button>

            {show_label.then(|| view! {
                <span class="c-subtitle">{label.clone()}</span>
            })}
            {show_hex.then(|| view! {
                <span class="c-colorpicker__hextext selectable">{move || current()}</span>
            })}

            // 点外关闭：铺一层透明的全屏垫子，压在井上但托着面板
            <div
                style=move || if open.get() {
                    format!("position:fixed;inset:0;z-index:{}", layer_z() - 1)
                } else {
                    "display:none".to_string()
                }
                on:click=move |_| close()
            ></div>

            <div
                role="dialog"
                aria-modal="false"
                aria-label=dialog_label
                tabindex="-1"
                node_ref=panel_ref
                style=move || if open.get() {
                    let (l, t) = pos.get();
                    format!("position:fixed;left:{l}px;top:{t}px;z-index:{}", layer_z())
                } else {
                    "display:none".to_string()
                }
            >
                <ColorPicker
                    value=Signal::derive(move || {
                        value.get().or_else(|| local.get()).unwrap_or_default()
                    })
                    presets=presets
                    label=label.clone()
                    on_change=Callback::new(move |hex| pick(hex))
                />
            </div>
        </div>
    }
}

/* ==========================================================================
   色块 Swatch / Swatches
   ========================================================================== */

/// 井的无障碍名 / 悬浮提示：颜色名 + 当前十六进制。
fn well_label(name: &str, hex: &str) -> String {
    if hex.is_empty() {
        name.to_string()
    } else {
        format!("{name} {hex}")
    }
}

/// 单色块。可选、可带名字；选中态走既有的 `aria-checked` 描边。
#[component]
pub fn Swatch(
    #[prop(into)] value: String,
    /// 名字（读屏与 title）；留空则念色值
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] selected: MaybeProp<bool>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_select: Option<Callback<String>>,
) -> impl IntoView {
    let name = if label.is_empty() {
        value.clone()
    } else {
        label.clone()
    };
    let checked = move || selected.get().unwrap_or(false);
    let emit = {
        let value = value.clone();
        move || {
            if let Some(cb) = on_select {
                cb.run(value.clone());
            }
        }
    };

    view! {
        <button
            class="c-swatch"
            type="button"
            role="radio"
            aria-checked=move || checked().to_string()
            aria-label=name.clone()
            title=name
            disabled=disabled
            style=format!("--sw:{value}")
            on:click=move |_| emit()
        ></button>
    }
}

/// 预设色网格。`selected` 是当前色值（十六进制，大小写不敏感）。
#[component]
pub fn Swatches(
    #[prop(into)] items: Vec<SwatchItem>,
    #[prop(into, optional)] selected: MaybeProp<String>,
    #[prop(into, optional)] label: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_select: Option<Callback<String>>,
) -> impl IntoView {
    let aria = if label.is_empty() {
        "预设颜色".to_string()
    } else {
        label
    };
    let select = Callback::new(move |v: String| {
        if let Some(cb) = on_select {
            cb.run(v);
        }
    });

    view! {
        <div class="c-swatches" role="radiogroup" aria-label=aria>
            {items
                .into_iter()
                .map(|item| {
                    let is_on = {
                        let value = item.value.clone();
                        move || {
                            selected
                                .get()
                                .map(|cur| cur.eq_ignore_ascii_case(&value))
                                .unwrap_or(false)
                        }
                    };
                    view! {
                        <Swatch
                            value=item.value
                            label=item.label
                            selected=Signal::derive(is_on)
                            disabled=disabled
                            on_select=select
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

/// 下一帧再执行。Leptos 的声明式重建在微任务里落定，焦点必须在它之后搬。
fn schedule(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb: js_sys::Function =
        Closure::once_into_js(Box::new(f) as Box<dyn FnOnce()>).unchecked_into();
    let _ = window.request_animation_frame(&cb);
}
