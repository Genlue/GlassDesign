//! 悬浮导航栏 Dock —— 侧边导航栏的另一种形态：一根浮在内容上的玻璃药丸。
//!
//! 为什么单独一个组件，而不是 NavColumn 的一个开关：Dock 是**浮在内容上的临时层**
//! （走玻璃），NavColumn 是**长期占位的窗格**（走系统磨砂）。两者的材质、定位、
//! 收起行为都不同，套在一起只会让两边都别扭。
//!
//! 选中语义跟 `40-nav-dock.css` 对齐：CSS 认的是 `aria-current="page"`（导航语义，
//! 和面包屑的当前项同一套），不是 `aria-selected` —— 这里照 CSS 写，不另立一套。
//! 因此 `.c-dock__pill` 在这份 CSS 里是**玻璃托盘**，不是滑动指示器；Dock 的
//! 「选中」就是选中那颗圆钮自己变成强调色实心（含过渡），不存在可位移的药丸。
//! 每颗圆钮仍写出 `--i` / `--n`，留给后续要做位移动画时直接用，不必再改 DOM。

use leptos::html;
use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};

/// 悬浮导航栏里的一颗圆钮。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DockItem {
    pub id: String,
    pub label: String,
    pub icon: IconName,
    pub badge: Option<String>,
}

impl DockItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            badge: None,
        }
    }

    /// 右上角的角标（未读数 / 待办数）。纯展示，不是第二个点击目标。
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// 停靠方向。对应 CSS 的 `data-edge`，三种都是既有规则：
/// 左 / 右是竖排（圆钮上下排），底部是横排（圆钮左右排）。
///
/// 键盘轴由它推出：竖排用 ↑/↓，横排用 ←/→ —— 和视觉方向一致，
/// 不然用户按「向上」而焦点横向跳，会读成 bug。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DockOrientation {
    #[default]
    Left,
    Right,
    Bottom,
}

impl DockOrientation {
    fn edge(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Bottom => "bottom",
        }
    }

    fn aria_orientation(self) -> &'static str {
        if self == Self::Bottom {
            "horizontal"
        } else {
            "vertical"
        }
    }
}

/// 悬浮导航栏。
///
/// - `sep_after`：在指定 id 的那一颗**之后**画一根组间分隔线（空串 = 不分隔）。
///   之所以不放进 `DockItem`：分隔线是排版边界，不属于某一颗按钮的数据。
/// - `foot`：药丸外面的独立圆钮（旧库的「重做」位），和药丸不是一体。
#[component]
pub fn NavDock(
    #[prop(into)] items: Vec<DockItem>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] on_select: Option<Callback<String>>,
    #[prop(optional)] orientation: DockOrientation,
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] sep_after: String,
    #[prop(optional)] foot: Option<DockItem>,
) -> impl IntoView {
    let n = items.len();
    let ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    let refs: Vec<NodeRef<html::Button>> = (0..n).map(|_| NodeRef::new()).collect();
    // 没有任何选中值时，第一颗拿 tabindex=0：否则整组从 Tab 序列里消失。
    let first_id = ids.first().cloned().unwrap_or_default();
    let aria = if label.is_empty() {
        "导航栏".to_string()
    } else {
        label
    };
    let aria_for_foot = aria.clone();

    let buttons = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let id_state = item.id.clone();
            let id_click = item.id.clone();
            let item_label = item.label.clone();
            let badge = item.badge.clone();
            let sep = !sep_after.is_empty() && sep_after == item.id;
            let btn_ref = refs[i];
            let ids_kb = ids.clone();
            let refs_kb = refs.clone();
            let first = first_id.clone();

            // roving tabindex：只有「当前项」在 Tab 序列里，其余靠方向键进。
            // 没有选中值时第一颗先占住 Tab 位 —— 但**不**标成 aria-current，
            // 「没选中任何一项」是一种合法状态，不能靠视觉/读屏假装选中了第一颗。
            // 两个属性各拿一份自己的捕获值：闭包不是 Copy，不能共用同一个。
            let id_current = id_state.clone();
            let id_tabindex = id_state;
            let first_tabindex = first;

            view! {
                <button
                    class="c-dock__item"
                    type="button"
                    data-dock-id=item.id.clone()
                    // 圆钮只有图标，可访问名必须显式给
                    aria-label=item_label.clone()
                    title=item_label
                    aria-current=move || {
                        value
                            .get()
                            .map(|v| v == id_current)
                            .unwrap_or(false)
                            .then_some("page")
                    }
                    tabindex=move || {
                        let on = match value.get() {
                            Some(v) => v == id_tabindex,
                            None => id_tabindex == first_tabindex,
                        };
                        if on { "0" } else { "-1" }
                    }
                    // 等宽假设的坐标，留给后续做位移动画（当前 CSS 不用它）
                    style=format!("--i:{i};--n:{n}")
                    node_ref=btn_ref
                    on:click={
                        let id = id_click.clone();
                        move |_| {
                            if let Some(cb) = on_select {
                                cb.run(id.clone());
                            }
                        }
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        let key = ev.key();
                        let step: isize = match (orientation, key.as_str()) {
                            (DockOrientation::Bottom, "ArrowRight") => 1,
                            (DockOrientation::Bottom, "ArrowLeft") => -1,
                            (DockOrientation::Left | DockOrientation::Right, "ArrowDown") => 1,
                            (DockOrientation::Left | DockOrientation::Right, "ArrowUp") => -1,
                            _ => 0,
                        };
                        if step != 0 {
                            ev.prevent_default();
                            let next = (i as isize + step).rem_euclid(ids_kb.len() as isize) as usize;
                            if let Some(el) = refs_kb[next].get() {
                                let _ = el.focus();
                            }
                            return;
                        }
                        match key.as_str() {
                            "Home" | "End" => {
                                ev.prevent_default();
                                let target = if key == "Home" { 0 } else { ids_kb.len() - 1 };
                                if let Some(el) = refs_kb[target].get() {
                                    let _ = el.focus();
                                }
                            }
                            // 手动激活：方向键只搬焦点，Enter/Space 才真的切页。
                            // 圆钮是导航目标，不是分段控件 —— 焦点路过就跳页会让人找不到路。
                            "Enter" | " " => {
                                ev.prevent_default();
                                if let Some(cb) = on_select {
                                    cb.run(ids_kb[i].clone());
                                }
                            }
                            _ => {}
                        }
                    }
                >
                    <Icon name=item.icon size=18 />
                    {badge.map(|b| view! { <span class="c-dock__badge">{b}</span> })}
                </button>
                // 分隔线跟在「这一颗之后」：sep_after 说的是组边界落在哪一颗后面，
                // 先画后画差一个位置，插在按钮前面就会把线落在错误的组之间。
                {sep.then(|| view! { <div class="c-dock__sep" role="presentation"></div> })}
            }
        })
        .collect_view();

    let foot_view = foot.map(|item| {
        let id = item.id.clone();
        view! {
            <div class="c-dock__foot">
                <button
                    class="c-dock__item"
                    type="button"
                    aria-label=item.label.clone()
                    title=item.label
                    style="--i:0;--n:1"
                    on:click=move |_| {
                        if let Some(cb) = on_select {
                            cb.run(id.clone());
                        }
                    }
                >
                    <Icon name=item.icon size=18 />
                    {item.badge.map(|b| view! { <span class="c-dock__badge">{b}</span> })}
                </button>
            </div>
        }
    });

    view! {
        <nav
            class="c-dock"
            aria-label=aria_for_foot
            data-edge=orientation.edge()
            // data-hidden 必须显式写 0：CSS 的收起态与 .l-stage 让位都是按
            // [data-hidden="0"] 命中的，缺这个属性两者的选择器都不成立。
            data-hidden="0"
        >
            <div class="c-dock__pill" role="toolbar" aria-orientation=orientation.aria_orientation()>
                {buttons}
            </div>
            {foot_view}
        </nav>
    }
}
