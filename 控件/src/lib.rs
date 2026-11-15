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

    pub use crate::widgets::avatar::{Avatar, AvatarSize, AvatarStatus};
    pub use crate::widgets::badge::{Badge, BadgeVariant};
    pub use crate::widgets::button::{Button, ButtonVariant, IconButton, IconButtonVariant};
    pub use crate::widgets::checkbox::Checkbox;
    pub use crate::widgets::breadcrumb::{Breadcrumb, Crumb};
    pub use crate::widgets::chip::Chip;
    pub use crate::widgets::dialog::Dialog;
    pub use crate::widgets::disclosure::Disclosure;
    pub use crate::widgets::field::Field;
    pub use crate::widgets::form_field::FormField;
    pub use crate::widgets::inline_message::{InlineMessage, Tone};
    pub use crate::widgets::list::{List, ListItem};
    pub use crate::widgets::number_field::NumberField;
    pub use crate::widgets::pagination::Pagination;
    pub use crate::widgets::radio_group::{RadioGroup, RadioOption};
    pub use crate::widgets::search_field::SearchField;
    pub use crate::widgets::select::{Select, SelectOption};
    pub use crate::widgets::segmented::{Segmented, SegmentedOption, SegmentedSize};
    pub use crate::widgets::slider::Slider;
    pub use crate::widgets::stepper::{Stepper, StepperOrientation};
    pub use crate::widgets::text_area::TextArea;
    pub use crate::widgets::toast::{toast, toast_with, ToastHost, ToastItem};
    pub use crate::widgets::progress::{Empty, Progress, ProgressRing, RingSize, Skeleton, Spinner};
    pub use crate::widgets::switch::Switch;
    pub use crate::widgets::tabs::{TabItem, Tabs};
    pub use crate::widgets::table::{SortDir, Table, TableColumn, TableRow, TableSort};

    pub use crate::layout::surfaces::{Card, Divider, Row, SectionTitle};
    pub use crate::layout::shell::{
        AppShell, Main, NavColumn, NavGroup, NavItem, NavSection, ScrollArea,
    };
    pub use crate::layout::toolbar::{Capsule, CapsuleSize, Toolbar, ToolbarLead, ToolbarPills, ToolbarTitle};
}

pub use prelude::*;
