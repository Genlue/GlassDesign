use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::base::anchor::{self, AnchorSpec, AnchorWatch};
use crate::base::overlay::{
    listen_document, GlobalListener, OverlayHost, OverlayId, OverlayKind,
};
use crate::widgets::color_picker::{hex_to_rgb, normalize_hex, ColorPicker, SwatchItem};

/* ==========================================================================
   颜色井 ColorWell

   井是控件，面板是浮层：面板走 `OverlayHost`（z 与 Esc 的唯一权威）+ 共享定位层
   `base::anchor`（贴锚点、越界翻转、视口夹取），并且 `Portal` 到 body —— 卡片的
   `overflow` 裁剪与祖先的 `transform` / `backdrop-filter` 都影响不到它。
   非模态，所以不 `bind`，也不做焦点陷阱。
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
    // 跟随句柄与点外监听：关闭时一起置 None，Drop 自动解绑。
    let watch = StoredValue::new_local(None::<AnchorWatch>);
    let outside = StoredValue::new_local(None::<GlobalListener>);
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
    let dialog_label = StoredValue::new_local(if label.is_empty() {
        "颜色取色器".to_string()
    } else {
        format!("{label}取色器")
    });
    // Portal 的 children 必须是 `Fn`：捕获的句柄必须 Copy，否则内层闭包每次
    // 构造都会把捕获值 move 出去，整个组件视图就退化成 FnOnce。
    let presets = StoredValue::new_local(presets);
    let label_for_panel = StoredValue::new_local(label.clone());

    // 关闭只有一条路：收起 → 解绑 → 出栈 → 回焦触发器。点外、Esc（host 回调）、
    // 再点一次井都走这里。
    let close = move || {
        open.set(false);
        watch.set_value(None);
        outside.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    };
    let dismiss = Callback::new(move |_| close());

    let open_panel = move || {
        if disabled || layer.get_untracked().is_some() {
            return;
        }
        // 只负责「开」：面板 Portal 到 body 之后再由 effect 量高度定位。
        let id = host.push(OverlayKind::Popover, dismiss);
        layer.set(Some(id));
        open.set(true);
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
        watch.set_value(None);
        outside.set_value(None);
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

    // 打开后定位：DOM 落定 → 量 → 摆 → 跟随。
    // 等一帧是因为面板的 style 闭包（负责第一帧 `visibility:hidden;left:-10000px`）
    // 在本 effect 之后才生效 —— 同帧先 apply 会被它整条覆盖回视口外。
    Effect::new(move |_| {
        let anchor_el = trigger_ref
            .get()
            .map(|b| b.unchecked_into::<web_sys::Element>());
        let panel = panel_ref.get();
        if !open.get() {
            watch.set_value(None);
            outside.set_value(None);
            return;
        }
        let (Some(anchor_el), Some(panel)) = (anchor_el, panel) else {
            // Portal 还没挂好：NodeRef 是响应式的，挂上后会再跑一遍。
            return;
        };

        // 点外关闭要同时认**触发器和面板**两个子树：面板已 Portal 到 body，
        // 不在组件 DOM 里，只查井会把「点面板内部」判成点外（SV 面板点不动）。
        // 拖拽期间没有新的 pointerdown，所以这条监听不会把拖拽误判成点外。
        if outside.with_value(|v| v.is_none()) {
            let trigger_key = trigger_ref;
            let panel_key = panel_ref;
            if let Some(listener) = listen_document("pointerdown", move |ev| {
                let Some(target) = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                let node: &web_sys::Node = target.unchecked_ref();
                let inside_trigger = trigger_key
                    .get()
                    .map(|t| t.contains(Some(node)))
                    .unwrap_or(false);
                let inside_panel = panel_key
                    .get()
                    .map(|p| p.contains(Some(node)))
                    .unwrap_or(false);
                if !inside_trigger && !inside_panel {
                    close();
                }
            }) {
                outside.set_value(Some(listener));
            }
        }

        schedule(move || {
            if !open.get_untracked() {
                return;
            }
            if !anchor_el.is_connected() || !panel.is_connected() {
                return;
            }
            // 取色器内容固定 236，留出 240 的最小宽度；不滚动、不跟井同宽。
            let spec = AnchorSpec {
                min_width: 240.0,
                scroll: false,
                ..AnchorSpec::bubble()
            };
            anchor::place_and_apply(&anchor_el, &panel, spec);
            // 焦点移进面板：非模态不禁背景，但「打开后键盘直接落在面板里」
            // 是浮层的常规预期。必须等定位完成 —— 第一帧面板是 visibility:hidden，
            // 直接 focus 会被浏览器忽略。
            if let Ok(Some(el)) = panel.query_selector(
                "[tabindex='0'], button:not([disabled]), input:not([disabled])",
            ) {
                if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
            // 锚点被滚出视口就收起：留一个挂在屏幕外的面板没有意义。
            let on_gone: Rc<dyn Fn()> = Rc::new(move || close());
            watch.set_value(Some(anchor::watch(anchor_el, panel.into(), spec, Some(on_gone))));
        });
    });

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

            // 点外关闭不再用全屏垫子：垫子留在带 transform 的祖先里会被缩成
            // 祖先那么大，点外面关不掉。改由下面的文档级 pointerdown 判定。
            <Portal>
                {move || view! {
                    <div
                        class="c-colorwell__panel"
                        role="dialog"
                        aria-modal="false"
                        aria-label=dialog_label.get_value()
                        tabindex="-1"
                        node_ref=panel_ref
                        style=move || if open.get() {
                            format!(
                                "z-index:{};position:fixed;left:-10000px;top:0px;visibility:hidden",
                                layer_z()
                            )
                        } else {
                            "display:none".to_string()
                        }
                    >
                        <ColorPicker
                            value=Signal::derive(move || {
                                value.get().or_else(|| local.get()).unwrap_or_default()
                            })
                            presets=presets.get_value()
                            label=label_for_panel.get_value()
                            on_change=Callback::new(move |hex| pick(hex))
                        />
                    </div>
                }}
            </Portal>
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
