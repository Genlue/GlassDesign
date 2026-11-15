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
    pub use crate::base::overlay::{OverlayHost, OverlayId, OverlayKind, Z_TOAST};
    pub use crate::base::theme::{Theme, ThemeMode};
    pub use crate::base::types::Size;

    pub use crate::widgets::avatar::{Avatar, AvatarSize, AvatarStatus};
    pub use crate::widgets::badge::{Badge, BadgeVariant};
    pub use crate::widgets::breadcrumb::{Breadcrumb, Crumb};
    pub use crate::widgets::button::{Button, ButtonVariant, IconButton, IconButtonVariant};
    pub use crate::widgets::checkbox::Checkbox;
    pub use crate::widgets::chip::Chip;
    pub use crate::widgets::color_picker::{hex_to_rgb, ColorPicker, SwatchItem};
    pub use crate::widgets::color_well::{ColorWell, Swatch, Swatches};
    pub use crate::widgets::collection_view::{
        CollectionItem, CollectionLayout, CollectionView,
    };
    pub use crate::widgets::combobox::{Combobox, ComboboxOption};
    pub use crate::widgets::data_table::{ColAlign, DataColumn, DataTable, DataTableRow};
    pub use crate::widgets::datetime::{DatePicker, TimePicker};
    pub use crate::widgets::dialog::Dialog;
    pub use crate::widgets::disclosure::Disclosure;
    pub use crate::widgets::document_tabs::{DocumentTab, DocumentTabs};
    pub use crate::widgets::drawer::{Drawer, DrawerSide};
    pub use crate::widgets::field::Field;
    pub use crate::widgets::file_picker::{DropZone, FileItem, FilePicker};
    pub use crate::widgets::form::{Form, FormActions};
    pub use crate::widgets::form_field::FormField;
    pub use crate::widgets::inline_message::{InlineMessage, Tone};
    pub use crate::widgets::inspector::{Inspector, Outline, OutlineItem, PropList, PropRow};
    pub use crate::widgets::list::{List, ListItem, ListSelection};
    pub use crate::widgets::media_tile::MediaTile;
    pub use crate::widgets::menu::{Menu, MenuButton, MenuItem};
    pub use crate::widgets::nav_card::NavCard;
    pub use crate::widgets::nav_dock::{DockItem, DockOrientation, NavDock};
    pub use crate::widgets::nav_info::NavInfo;
    pub use crate::widgets::nav_search::NavSearch;
    pub use crate::widgets::number_field::NumberField;
    pub use crate::widgets::pagination::Pagination;
    pub use crate::widgets::popover::{Popover, PopoverPlacement};
    pub use crate::widgets::progress::{
        Empty, Progress, ProgressRing, RingSize, Skeleton, Spinner,
    };
    pub use crate::widgets::radio_group::{RadioGroup, RadioOption};
    pub use crate::widgets::range_slider::RangeSlider;
    pub use crate::widgets::search_field::SearchField;
    pub use crate::widgets::segmented::{Segmented, SegmentedOption, SegmentedSize};
    pub use crate::widgets::select::{Select, SelectOption};
    pub use crate::widgets::selection::SelectionMode;
    pub use crate::widgets::sheet::{Sheet, SheetSize};
    pub use crate::widgets::slider::Slider;
    pub use crate::widgets::split_view::{SplitOrientation, SplitView};
    pub use crate::widgets::status_bar::{StatusBar, StatusItem, StatusProgress, StatusTone};
    pub use crate::widgets::stepper::{Stepper, StepperOrientation};
    pub use crate::widgets::switch::Switch;
    pub use crate::widgets::table::{SortDir, Table, TableColumn, TableRow, TableSort};
    pub use crate::widgets::tabs::{TabItem, Tabs};
    pub use crate::widgets::text_area::TextArea;
    pub use crate::widgets::tile::Tile;
    pub use crate::widgets::toast::{toast, toast_with, ToastHost, ToastItem};
    pub use crate::widgets::tooltip::{Tooltip, TooltipPlacement};
    pub use crate::widgets::token_field::TokenField;
    pub use crate::widgets::toolbar_customization::{ToolbarCustomization, ToolbarField};
    pub use crate::widgets::toolbar_overflow::{ToolbarAction, ToolbarOverflow};
    pub use crate::widgets::tree_view::{TreeNode, TreeState, TreeView};

    pub use crate::layout::glass_panel::{GlassPanel, GlassPanelSize};
    pub use crate::layout::shell::{
        AppShell, Main, NavColumn, NavGroup, NavItem, NavSection, ScrollArea,
    };
    pub use crate::layout::surfaces::{Card, Divider, Row, SectionTitle};
    pub use crate::layout::toolbar::{
        Capsule, CapsuleSize, Toolbar, ToolbarLead, ToolbarPills, ToolbarTitle,
    };
}

pub use prelude::*;
