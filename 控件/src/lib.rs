//! GlassDesign · Tauri 2 + Leptos 控件库
//!
//! 用法：
//! ```ignore
//! glass_design::bootstrap(App);
//! ```
//!
//! 或者拆开手写：`install()` → `Theme::provide()` → `mount_to_body()`。

// 重新导出框架本体，给 `Signal` / `Callback` 这类 props 类型一个版本锚点，
// 也让 `glass_design::leptos` 在泛型与文档里可引用。
// 注意：它替代不了直接依赖 —— `#[component]` / `view!` 展开成 `::leptos::…` 的绝对路径，
// 使用方仍要在自己的 Cargo.toml 里声明 leptos 与 wasm-bindgen（见 README §1）。
pub use leptos;
pub use wasm_bindgen;

pub mod base;
pub mod layout;
pub mod widgets;

pub mod prelude {
    pub use crate::base::bootstrap::*;
    pub use crate::base::i18n::{fill, provide_locale, t, Locale, Strings};
    pub use crate::base::icon::{Icon, IconName, ICON_COUNT};
    pub use crate::base::install::{
        glass_defs_svg, install, install_with, styles_css, InstallConfig, InstallReport, VERSION,
    };
    pub use crate::base::overlay::{OverlayHost, OverlayId, OverlayKind, Z_TOAST};
    pub use crate::base::scrollbar::{attach as attach_scrollbar, ScrollbarHandle};
    pub use crate::base::theme::{Theme, ThemeMode};
    pub use crate::base::types::Size;

    pub use crate::widgets::audio_spectrum::{AudioSpectrum, SpectrumSize};
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
    pub use crate::widgets::cover_art::{CoverArt, CoverArtSize};
    pub use crate::widgets::data_table::{
        CellEdit, ColAlign, DataColumn, DataTable, DataTableRow, SortSpec,
    };
    pub use crate::widgets::datetime::{DatePicker, DateRange, TimePicker, TimeRange};
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
    pub use crate::widgets::lyric_view::{LyricLine, LyricView};
    pub use crate::widgets::media_icons::media_icons;
    pub use crate::widgets::media_tile::MediaTile;
    pub use crate::widgets::menu::{Menu, MenuButton, MenuItem};
    pub use crate::widgets::nav_card::NavCard;
    pub use crate::widgets::nav_dock::{DockItem, DockOrientation, NavDock};
    pub use crate::widgets::nav_info::NavInfo;
    pub use crate::widgets::nav_search::NavSearch;
    pub use crate::widgets::number_field::NumberField;
    pub use crate::widgets::pagination::Pagination;
    pub use crate::widgets::player_bar::{PlayMode, PlayerBar, PlayerTrackInfo};
    pub use crate::widgets::popover::{Popover, PopoverPlacement};
    pub use crate::widgets::progress::{
        Empty, Progress, ProgressRing, RingSize, Skeleton, Spinner,
    };
    pub use crate::widgets::radio_group::{RadioGroup, RadioOption};
    pub use crate::widgets::range_slider::RangeSlider;
    pub use crate::widgets::search_field::SearchField;
    pub use crate::widgets::section_wave::{SectionWave, WaveItem};
    pub use crate::widgets::segmented::{Segmented, SegmentedOption, SegmentedSize};
    pub use crate::widgets::select::{Select, SelectOption};
    pub use crate::widgets::selection::SelectionMode;
    pub use crate::widgets::sheet::{Sheet, SheetSize};
    pub use crate::widgets::slider::Slider;
    pub use crate::widgets::split_grip::SplitGrip;
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
    pub use crate::widgets::track_list::{TrackItem, TrackList};
    pub use crate::widgets::traffic_lights::TrafficLights;
    pub use crate::widgets::token_field::TokenField;
    pub use crate::widgets::toolbar_customization::{ToolbarCustomization, ToolbarField};
    pub use crate::widgets::toolbar_overflow::{ToolbarAction, ToolbarOverflow};
    pub use crate::widgets::tree_view::{TreeNode, TreeState, TreeView};

    pub use crate::layout::glass_panel::{GlassPanel, GlassPanelSize};
    pub use crate::layout::primitives::*;
    pub use crate::layout::shell::{
        AppShell, Main, NavColumn, NavGroup, NavItem, NavSection, ScrollArea,
    };
    pub use crate::layout::surfaces::{Card, Divider, Row, SectionTitle};
    pub use crate::layout::toolbar::{
        Capsule, CapsuleSize, NavPair, Toolbar, ToolbarChrome, ToolbarLead, ToolbarPills,
        ToolbarTitle,
    };
}

pub use prelude::*;
