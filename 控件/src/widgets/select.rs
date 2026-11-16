use std::rc::Rc;

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::base::anchor::{self, AnchorSpec, AnchorWatch};
use crate::base::i18n::t;
use crate::base::icon::{Icon, IconName};
use crate::base::overlay::{schedule, OverlayHost, OverlayId, OverlayKind};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
}

#[component]
pub fn Select(
    #[prop(into)] options: Vec<SelectOption>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] width: u32,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let trigger_ref = NodeRef::<html::Button>::new();
    // 面板 Portal 到 body，已经不在组件 DOM 里：只能自己拿 NodeRef 去量。
    let panel_ref = NodeRef::<html::Div>::new();
    let width = if width == 0 { 180 } else { width };
    // 栈句柄与打开状态同步：z-index / Esc 由 host 决定，定位交给 base::anchor。
    let host = OverlayHost::use_or_provide();
    let layer = RwSignal::new(None::<OverlayId>);
    // 跟随句柄：关闭时 take 掉，Drop 自动解绑滚动 / resize / 尺寸监听。
    let watch = StoredValue::new_local(None::<AnchorWatch>);
    // Portal 的 children 必须是 `Fn`：捕获的句柄必须 Copy，否则内层闭包每次
    // 构造都会把捕获值 move 出去，整个组件视图就退化成 FnOnce。
    let options_for_menu = StoredValue::new_local(options.clone());
    let options_for_label = options.clone();
    let current_label = move || {
        let v = value.get().unwrap_or_default();
        options_for_label
            .iter()
            .find(|o| o.value == v)
            .map(|o| o.label.clone())
            .unwrap_or_default()
    };
    let has_value = move || !value.get().unwrap_or_default().is_empty();

    // 关闭只有一条路：出栈 + 收起。点外、选中、Esc（host 回调）都走这里。
    let close = move || {
        // 先收起、再出栈：面板的 style 闭包依赖 open，先关掉能保证随后的
        // 栈变化只会重写成 `display:none`，不会擦掉已经写好的坐标。
        open.set(false);
        watch.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
            layer.set(None);
        }
    };
    let dismiss = Callback::new(move |_| close());

    // 打开只负责「开」：定位要等面板真的挂进 body、量得到高度之后。
    let open_menu = move || {
        if open.get_untracked() || layer.get_untracked().is_some() {
            return;
        }
        // `push_adopted`：在气泡 / 流程面板里时挂到容器那层下面，否则等价 `push`。
        let id = host.push_adopted(OverlayKind::Menu, dismiss);
        layer.set(Some(id));
        open.set(true);
    };

    on_cleanup(move || {
        watch.set_value(None);
        if let Some(id) = layer.get_untracked() {
            host.remove(id);
        }
    });

    let menu_z = move || {
        layer
            .get()
            .map(|id| host.z(id))
            .unwrap_or(OverlayKind::Menu.base_z())
    };

    // 打开后定位：DOM 落定 → 量 → 摆 → 跟随。
    // 这里再等一帧（rAF），是因为面板的 style 闭包（负责第一帧的
    // `visibility:hidden;left:-10000px`）在本 effect 之后才生效 —— 同一帧里
    // 先 apply 会被它整条 style 覆盖回视口外。等一帧后 style 已落定，apply 才留得住。
    Effect::new(move |_| {
        let anchor_el = trigger_ref
            .get()
            .map(|b| b.unchecked_into::<web_sys::Element>());
        let panel = panel_ref.get();
        if !open.get() {
            watch.set_value(None);
            return;
        }
        let (Some(anchor_el), Some(panel)) = (anchor_el, panel) else {
            // Portal 还没挂好：NodeRef 是响应式的，挂上后会再跑一遍。
            return;
        };
        // 把面板交给栈：外层气泡的「点外关闭」要认出「这一下点的是我的子面板」。
        if let Some(id) = layer.get_untracked() {
            host.bind(id, panel.clone().into());
        }
        schedule(move || {
            // 这一帧里可能已经关了（快速开→关）：别再建一个新的跟随句柄，
            // 否则关闭后监听器还活着，下一个滚动事件又会把它摆回来。
            if !open.get_untracked() {
                return;
            }
            if !anchor_el.is_connected() || !panel.is_connected() {
                return;
            }
            let spec = AnchorSpec::dropdown();
            anchor::place_and_apply(&anchor_el, &panel, spec);
            // 锚点被滚出视口就收起：留一个挂在屏幕外的面板没有意义。
            let on_gone: Rc<dyn Fn()> = Rc::new(move || close());
            watch.set_value(Some(anchor::watch(anchor_el, panel.into(), spec, Some(on_gone))));
        });
    });

    // 面板的「第一帧」样式：还没定位时藏到视口外，等 anchor::apply 显形。
    // 定位完成后 apply 只逐条写内联属性，不会经过这条闭包。
    let panel_style = move || {
        if open.get() {
            format!(
                "z-index:{};position:fixed;left:-10000px;top:0px;visibility:hidden",
                menu_z()
            )
        } else {
            "display:none".to_string()
        }
    };

    view! {
        <div class="c-select" data-open=move || open.get().then_some("1") style=format!("position:relative;width:{width}px")>
            <button
                class="c-btn"
                type="button"
                aria-haspopup="menu"
                aria-label=label
                aria-expanded=move || open.get().to_string()
                style="width:100%;justify-content:space-between"
                node_ref=trigger_ref
                on:click=move |_| {
                    if open.get_untracked() {
                        close();
                    } else {
                        open_menu();
                    }
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    // Esc 交给 OverlayHost：输入法组合态、只关最上层都在那边处理。
                    if ev.key() == "ArrowDown" || ev.key() == "ArrowUp" {
                        ev.prevent_default();
                        open_menu();
                    }
                }
            >
                <span class="l-fill" style="text-align:left">
                    {move || if has_value() { current_label() } else { t().select_placeholder.to_string() }}
                </span>
                <span style="display:grid" style:transform=move || format!("rotate({}deg)", if open.get() { 180 } else { 0 })>
                    <Icon name=IconName::CHEVRON_DOWN size=12 />
                </span>
            </button>
        </div>

        // 遮罩也 Portal 到 body：`inset:0` 只有在视口坐标下才是全屏。
        // 留在组件里时，带 transform 的祖先会把它缩成祖先那么大，点外面关不掉。
        // z 序：遮罩 = z-1，面板 = z —— 点面板里的选项不会被当成点外。
        <Portal>
            {move || {
                open.get().then(|| {
                    let z = menu_z() - 1;
                    view! {
                        <div
                            style=format!("position:fixed;inset:0;z-index:{z}")
                            on:click=move |_| close()
                        ></div>
                    }
                })
            }}
        </Portal>

        // 面板必须到 body 下：`.c-field` / 卡片上的 backdrop-filter、transform
        // 会给 position:fixed 造包含块，面板会按祖先坐标摆、跟着容器滚。
        <Portal>
            {move || {
                // 每次调用取一份快照：句柄是 Copy，闭包因此仍是 `Fn`。
                let options = options_for_menu.get_value();
                let items = options
                    .iter()
                    .map(|opt| {
                        let opt_value = opt.value.clone();
                        let opt_label = opt.label.clone();
                        let disabled = opt.disabled;
                        let v1 = opt_value.clone();
                        let v2 = opt_value.clone();
                        view! {
                            <div
                                class="c-menu__item"
                                role="menuitemradio"
                                tabindex="-1"
                                data-label=opt_label.clone()
                                aria-checked=move || (value.get().as_deref() == Some(v1.as_str())).to_string()
                                aria-disabled=disabled.then_some("true")
                                on:click=move |_| {
                                    if disabled {
                                        return;
                                    }
                                    if let Some(cb) = on_change {
                                        cb.run(opt_value.clone());
                                    }
                                    close();
                                }
                            >
                                <span class="c-menu__check" data-role="menuitemradio">
                                    {move || (value.get().as_deref() == Some(v2.as_str())).then(|| view! { <Icon name=IconName::CHECK size=12 /> })}
                                </span>
                                <span class="c-menu__label">{opt_label.clone()}</span>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>();

                view! {
                    <div
                        class="c-menu"
                        role="menu"
                        tabindex="-1"
                        node_ref=panel_ref
                        style=panel_style
                    >
                        {items}
                    </div>
                }
            }}
        </Portal>
    }
}
