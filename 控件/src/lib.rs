//! GlassDesign · Tauri 2 + Leptos 控件库
//!
//! 用法：
//! ```ignore
//! glass_design::install();
//! let theme = glass_design::Theme::provide();
//! ```

pub mod base;
pub mod layout;
pub mod widgets;

pub mod prelude {
    pub use crate::base::icon::{Icon, IconName, ICON_COUNT};
    pub use crate::base::install::{install, InstallReport, VERSION};
    pub use crate::base::theme::{Theme, ThemeMode};
    pub use crate::base::types::Size;

    pub use crate::widgets::button::{Button, ButtonVariant, IconButton, IconButtonVariant};
    pub use crate::widgets::field::Field;
    pub use crate::widgets::switch::Switch;

    pub use crate::layout::surfaces::{Card, Divider, Row, SectionTitle};
    pub use crate::layout::shell::{
        AppShell, Main, NavColumn, NavGroup, NavItem, NavSection, ScrollArea,
    };
    pub use crate::layout::toolbar::{Capsule, CapsuleSize, Toolbar, ToolbarLead, ToolbarPills, ToolbarTitle};
}

pub use prelude::*;
