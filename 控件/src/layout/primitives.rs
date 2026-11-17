//! 布局原语：把 `.l-*` 那些空间类包成组件。
//!
//! 以前拼页面要写裸 `<div class="l-stack">`，等于在声明式组件里开了一个手写 HTML 的口子。
//! 这里每个组件只做一件事：挂对应的 `l-*` 类，并允许调用方追加自己的类名。
//!
//! 语义以 `控件/css/src/layout.css` 为准 —— 组件不新增类，只转发。

use leptos::prelude::*;

/// 拼类名：组件自身的类在前，调用方的类追加在后。
/// 只拼接不去重 —— 重复的类对 CSS 无副作用，而保留调用方的类才能让它按需覆盖。
fn with_class(base: &str, extra: &str) -> String {
    if extra.is_empty() {
        base.to_string()
    } else {
        format!("{base} {extra}")
    }
}

/// 纵向堆叠。间距由 `.l-stack` 的 `gap` 决定。
///
/// 需要「一列东西从上往下排」时用它，而不是裸写 `<div style="display:flex">`：
/// 方向与间距是排版约定，交给样式表统一，页面里不再出现布局属性。
/// `fill` 让它在父级 flex 里吃掉剩余空间（配合 `Push` 之外的场景，比如占满主区）。
#[component]
pub fn Stack(
    children: Children,
    #[prop(into, optional)] class: String,
    #[prop(optional)] fill: bool,
) -> impl IntoView {
    let cls = with_class(if fill { "l-stack l-fill" } else { "l-stack" }, &class);
    view! { <div class=cls>{children()}</div> }
}

/// 横排。子项默认垂直居中，`between` / `end` 控制主轴对齐。
///
/// 「图标 + 文字 + 右侧按钮」这类一行内的东西用它，而不是裸写 flex 容器：
/// 居中与最小宽度（`min-width: 0`，长文本可省略）已经是社区的默认解法。
/// `between` 把两端顶开（左标题右操作），`end` 全部靠右（对话框底部按钮）。
#[component]
pub fn Cluster(
    children: Children,
    #[prop(into, optional)] class: String,
    #[prop(optional)] between: bool,
    #[prop(optional)] end: bool,
) -> impl IntoView {
    let mut cls = String::from("l-row");
    if between {
        cls.push_str(" l-row--between");
    }
    if end {
        cls.push_str(" l-row--end");
    }
    let cls = with_class(&cls, &class);
    view! { <div class=cls>{children()}</div> }
}

/// 网格。列数与间距由 `--grid-gap` 等令牌控制，行高按内容，顶部对齐。
///
/// 卡片墙、图标宫格用它，而不是裸写 `display:grid`：网格的间距来自令牌，
/// 页面里写 `gap:16px` 就等于把设计规范复制了一份。
#[component]
pub fn Grid(children: Children, #[prop(into, optional)] class: String) -> impl IntoView {
    let cls = with_class("l-grid", &class);
    view! { <div class=cls>{children()}</div> }
}

/// 整页容器：撑满可用高度、自成裁剪边界，是路由切换的那一层。
///
/// 一个路由视图的最外层用它，而不是裸写 `<div>`：`.l-page` 负责 `height:100%`
/// 与 `overflow:hidden`，页面内部的滚动才能落到该滚的那一段上。
/// 顶栏是浮层、不占列 —— 没有套 `Stage` 的整页视图要再加 `PageInset` 让出那一行。
#[component]
pub fn Page(children: Children, #[prop(into, optional)] class: String) -> impl IntoView {
    let cls = with_class("l-page", &class);
    view! { <div class=cls>{children()}</div> }
}

/// 整页视图的顶边留白：让出被顶栏（`--toolbar-h`）压住的那一行。
///
/// 只给「自己管滚动、没有套 `Stage`」的页面用。`Stage` 已内含同一条留白，
/// 两者只取其一 —— 叠加会多出一条空白。
#[component]
pub fn PageInset(children: Children, #[prop(into, optional)] class: String) -> impl IntoView {
    let cls = with_class("l-page-inset", &class);
    view! { <div class=cls>{children()}</div> }
}

/// 滚动舞台：主区里真正滚动的那个框，整页视图的落点。
///
/// 用 `Main` 下的内容区时套它，而不是给内容自己加 `overflow:auto`：
/// 舞台已经绝对定位铺满主区、让出顶栏、并处理了过度滚动（`overscroll-behavior`）。
/// `flush` 撤掉顶部留白，给「首屏就是一张图 / 一个自定义头部」的页面用。
#[component]
pub fn Stage(
    children: Children,
    #[prop(into, optional)] class: String,
    #[prop(optional)] flush: bool,
) -> impl IntoView {
    let cls = with_class(
        if flush { "l-stage l-stage--flush" } else { "l-stage" },
        &class,
    );
    view! { <div class=cls>{children()}</div> }
}

/// 居中：把子项在横纵两个方向都摆到正中，并撑满父级高度。
///
/// 空态、加载态、登录框这类「一块内容居中」的页面用它，而不是自己算
/// `display:grid; place-items:center` —— 高度取满与最小高度归零都在类里。
#[component]
pub fn Center(children: Children, #[prop(into, optional)] class: String) -> impl IntoView {
    let cls = with_class("l-center", &class);
    view! { <div class=cls>{children()}</div> }
}

/// 撑开的空节点：在 `Cluster` 里把后面的东西推到最右。
///
/// 想在「标题 | 操作」之间留一个可伸缩的空隙时用它，而不是给操作加 `margin-left:auto`：
/// 推挤是容器的事，操作按钮不该知道自己站在哪。
#[component]
pub fn Push(#[prop(into, optional)] class: String) -> impl IntoView {
    let cls = with_class("l-push", &class);
    view! { <div class=cls></div> }
}

/// 填充：在父级 flex 里吃掉剩余空间，并允许子内容收缩（`min-width: 0`）。
///
/// 「侧栏固定 + 内容自适应」里的内容列用它，而不是写 `flex:1`：
/// 少了最小宽度归零，长路径或长单词会把整列撑爆。
#[component]
pub fn Fill(children: Children, #[prop(into, optional)] class: String) -> impl IntoView {
    let cls = with_class("l-fill", &class);
    view! { <div class=cls>{children()}</div> }
}

/// 内边距：给一段内容套上统一的内容边距。`wide` 是更松的版本（顶底与左右不同）。
///
/// 页面/卡片里包一层内容时用它，而不是给每个子元素写 `padding`：
/// 边距是一套账，改一处就该全站一起动。
#[component]
pub fn Pad(
    children: Children,
    #[prop(into, optional)] class: String,
    #[prop(optional)] wide: bool,
) -> impl IntoView {
    let cls = with_class(if wide { "l-pad l-pad--wide" } else { "l-pad" }, &class);
    view! { <div class=cls>{children()}</div> }
}
