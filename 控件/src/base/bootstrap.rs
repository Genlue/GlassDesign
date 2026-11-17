//! 应用入口：把「注入样式 + 建主题 + 开浮层栈 + 起语言」收成一次调用。
//!
//! 以前每个项目都要自己排这几行，漏一行就是没样式或没主题：
//!
//! ```ignore
//! glass_design::install();
//! let theme = glass_design::Theme::provide();
//! leptos::mount::mount_to_body(App);
//! ```
//!
//! 现在只剩一句：
//!
//! ```ignore
//! glass_design::bootstrap(App);
//! ```
//!
//! 要改默认主题 / 语言、或者样式自己 `<link>`，用 [`bootstrap_with`]：
//!
//! ```ignore
//! glass_design::bootstrap_with(App, BootstrapOptions {
//!     install: false,
//!     theme: ThemeMode::Dark,
//!     ..Default::default()
//! });
//! ```
//!
//! 手写壳的场合（要把 `<GlassApp>` 放在自己的根节点里）也能拆开用。

use leptos::prelude::*;

use crate::base::i18n::{provide_locale, Locale};
use crate::base::install::install;
use crate::base::overlay::OverlayHost;
use crate::base::theme::{Theme, ThemeMode};

/// 入口的默认值。字段少但都常用：主题三态、强调色、界面语言。
#[derive(Clone, Copy, Debug)]
pub struct BootstrapOptions {
    /// 是否注入 `css/glass.css` 与 `glass-defs.svg`。默认 `true`。
    ///
    /// 关掉它等于「样式自己 `<link>`」：wasm 少 465 KB，首帧不用等 wasm 才上样式。
    pub install: bool,
    /// 初始主题模式。默认跟随系统。
    pub theme: ThemeMode,
    /// 强调色。默认值必须与 [`Theme::provide`] 里的初值一致，
    /// 否则同一个应用「走 bootstrap」和「手写 Theme::provide」会差一个蓝。
    pub accent: &'static str,
    /// 界面语言。默认中文，与不调用 `provide_locale` 时的行为一致。
    pub locale: Locale,
}

impl Default for BootstrapOptions {
    fn default() -> Self {
        Self {
            install: true,
            theme: ThemeMode::System,
            // 与 Theme::provide() 的 accent 初值同步。
            accent: "#0a84ff",
            locale: Locale::ZhCn,
        }
    }
}

/// 应用根容器：注入样式、提供主题与浮层栈，然后把 children 渲染出来。
///
/// 它是 `bootstrap()` 内部用的那一层，也可以直接写进自己的根节点。
#[component]
pub fn GlassApp(children: Children) -> impl IntoView {
    install();
    let _theme = Theme::provide();
    let _host = OverlayHost::use_or_provide();
    children()
}

/// 装上库并挂载根组件。等价于 `install()` + `Theme::provide()` + `mount_to_body`。
///
/// 这里不套 `<GlassApp>`：`Children` 这一槽在 0.8 要求 `Send`，套一层会把
/// 「根组件捕获了 `Rc`」这种正常写法挡在门外。直接在根 owner 里铺好上下文，
/// 再把 `app()` 的产物交回去，效果一样而约束更少。
pub fn bootstrap<F, N>(app: F)
where
    F: FnOnce() -> N + 'static,
    N: IntoView + 'static,
{
    bootstrap_with(app, BootstrapOptions::default())
}

/// 与 [`bootstrap`] 相同，但默认值可改。
///
/// 顺序是有意的：先把样式落进 DOM，再挂载 —— 挂载过程里组件量尺寸
/// （胶囊贴图、波浪条）时样式已经在位，不会拿一帧裸 HTML 去算。
pub fn bootstrap_with<F, N>(app: F, opts: BootstrapOptions)
where
    F: FnOnce() -> N + 'static,
    N: IntoView + 'static,
{
    if opts.install {
        install();
    }

    leptos::mount::mount_to_body(move || {
        let theme = Theme::provide();
        theme.set_mode(opts.theme);
        theme.set_accent(opts.accent);
        // `new_local`：语言信号只活在当前线程，不需要 Send/Sync，
        // 也就不给挂在它上面的组件加约束。
        provide_locale(RwSignal::new_local(opts.locale));
        let _host = OverlayHost::use_or_provide();
        app()
    });
}
