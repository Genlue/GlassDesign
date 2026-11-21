use glass_design::layout::shell::NavItemProps;
use glass_design::prelude::*;
use glass_design::widgets::{
    badge::BadgeProps, button::ButtonProps, menu::MenuButtonProps, nav_card::NavCardProps,
    status_bar::StatusBarProps,
};
use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::props::{error, Props};
use crate::Emitter;

pub(crate) enum Widget {
    Button {
        label: String,
        icon: Option<IconName>,
        variant: ButtonVariant,
        size: Size,
        disabled: bool,
        class: String,
    },
    IconButton {
        label: String,
        icon: IconName,
        variant: IconButtonVariant,
        size: Size,
        disabled: bool,
        active: bool,
        toggle: bool,
        class: String,
    },
    NavItem {
        label: String,
        icon: Option<IconName>,
        selected: bool,
        count: Option<u32>,
        disabled: bool,
    },
    NavCard {
        title: String,
        desc: String,
        icon: Option<IconName>,
    },
    Segmented {
        label: String,
        options: Vec<SegmentedOption>,
        value: String,
    },
    SearchField {
        value: String,
        label: String,
        placeholder: String,
        width: u32,
        disabled: bool,
        id: String,
    },
    Select {
        options: Vec<SelectOption>,
        value: String,
        label: String,
        width: u32,
    },
    InlineMessage {
        tone: Tone,
        title: String,
        message: String,
    },
    Badge {
        label: String,
        variant: BadgeVariant,
        icon: Option<IconName>,
    },
    Card {
        class: String,
    },
    StatusBar,
    Toolbar,
    Capsule {
        size: CapsuleSize,
        class: String,
    },
    GlassPanel {
        size: Option<GlassPanelSize>,
    },
    SplitGrip {
        value: u32,
        min: u32,
        max: u32,
        label: String,
        controls: String,
    },
    TrafficLights {
        focused: bool,
        maximized: bool,
        dirty: bool,
        reveal: bool,
    },
    Menu {
        label: String,
        icon: Option<IconName>,
        class: String,
        items: Vec<MenuItem>,
    },
}

impl Widget {
    pub fn parse(kind: &str, props: &JsValue) -> Result<Self, JsValue> {
        let p = Props::new(props, kind)?;
        Ok(match kind {
            "button" => Self::Button {
                label: p.text("label")?,
                icon: p.icon()?,
                size: p.size()?,
                disabled: p.flag("disabled")?,
                class: p.text("className")?,
                variant: p.choice(
                    "variant",
                    ButtonVariant::Secondary,
                    &[
                        ("default", ButtonVariant::Secondary),
                        ("secondary", ButtonVariant::Secondary),
                        ("primary", ButtonVariant::Primary),
                        ("ghost", ButtonVariant::Plain),
                        ("plain", ButtonVariant::Plain),
                        ("danger", ButtonVariant::Danger),
                        ("tonal", ButtonVariant::Tonal),
                    ],
                )?,
            },
            "icon-button" => Self::IconButton {
                label: p.required_text("label")?,
                icon: p
                    .icon()?
                    .ok_or_else(|| error("icon-button.icon is required"))?,
                size: p.size()?,
                disabled: p.flag("disabled")?,
                active: p.flag("active")?,
                toggle: p.flag("toggle")?,
                class: p.text("className")?,
                variant: p.choice(
                    "variant",
                    IconButtonVariant::Plain,
                    &[
                        ("default", IconButtonVariant::Plain),
                        ("plain", IconButtonVariant::Plain),
                        ("ghost", IconButtonVariant::Plain),
                        ("tonal", IconButtonVariant::Tonal),
                        ("on", IconButtonVariant::On),
                    ],
                )?,
            },
            "nav-item" => Self::NavItem {
                label: p.text("label")?,
                icon: p.icon()?,
                selected: p.flag("selected")?,
                count: p.uint("count")?,
                disabled: p.flag("disabled")?,
            },
            "nav-card" => Self::NavCard {
                title: p.text("title")?,
                desc: p.text("desc")?,
                icon: p.icon()?,
            },
            "segmented" => {
                let options = p.list("options", |o| {
                    Ok(SegmentedOption {
                        value: o.required_text("value")?,
                        label: o.required_text("label")?,
                        icon: o.icon()?,
                        title: None,
                    })
                })?;
                let value = p.optional_text("value")?.unwrap_or_else(|| {
                    options.first().map(|o| o.value.clone()).unwrap_or_default()
                });
                Self::Segmented {
                    label: p.text("label")?,
                    options,
                    value,
                }
            }
            "search-field" => Self::SearchField {
                value: p.text("value")?,
                label: p.text("label")?,
                placeholder: p.text("placeholder")?,
                width: p.uint("width")?.unwrap_or_default(),
                disabled: p.flag("disabled")?,
                id: p.text("id")?,
            },
            "select" => Self::Select {
                options: p.list("options", |o| {
                    Ok(SelectOption {
                        value: o.required_text("value")?,
                        label: o.required_text("label")?,
                        disabled: o.flag("disabled")?,
                    })
                })?,
                value: p.text("value")?,
                label: p.text("label")?,
                width: p.uint("width")?.unwrap_or_default(),
            },
            "inline-message" => Self::InlineMessage {
                title: p.text("title")?,
                message: p.text("message")?,
                tone: p.choice(
                    "tone",
                    Tone::Info,
                    &[
                        ("info", Tone::Info),
                        ("success", Tone::Success),
                        ("warning", Tone::Warning),
                        ("danger", Tone::Danger),
                        ("error", Tone::Danger),
                    ],
                )?,
            },
            "badge" => Self::Badge {
                label: p.required_text("label")?,
                icon: p.icon()?,
                variant: p.choice(
                    "variant",
                    BadgeVariant::Neutral,
                    &[
                        ("neutral", BadgeVariant::Neutral),
                        ("accent", BadgeVariant::Accent),
                        ("danger", BadgeVariant::Danger),
                    ],
                )?,
            },
            "card" => Self::Card {
                class: p.text("className")?,
            },
            "status-bar" => Self::StatusBar,
            "toolbar" => Self::Toolbar,
            "capsule" => Self::Capsule {
                size: p.choice(
                    "size",
                    CapsuleSize::Auto,
                    &[
                        ("auto", CapsuleSize::Auto),
                        ("sm", CapsuleSize::Sm),
                        ("md", CapsuleSize::Md),
                        ("round", CapsuleSize::Round),
                    ],
                )?,
                class: p.text("className")?,
            },
            "glass-panel" => Self::GlassPanel {
                size: p.choice(
                    "size",
                    None,
                    &[
                        ("auto", None),
                        ("320x220", Some(GlassPanelSize::P320x220)),
                        ("480x320", Some(GlassPanelSize::P480x320)),
                        ("640x420", Some(GlassPanelSize::P640x420)),
                    ],
                )?,
            },
            "split-grip" => Self::SplitGrip {
                value: p.uint("value")?.unwrap_or_default(),
                min: p.uint("min")?.unwrap_or_default(),
                max: p.uint("max")?.unwrap_or(600),
                label: p.required_text("label")?,
                controls: p.text("controls")?,
            },
            "traffic-lights" => Self::TrafficLights {
                focused: p.optional_flag("focused")?.unwrap_or(true),
                maximized: p.flag("maximized")?,
                dirty: p.flag("dirty")?,
                reveal: p.flag("reveal")?,
            },
            "menu" => Self::Menu {
                label: p.text("label")?,
                icon: p.icon()?,
                class: p.text("className")?,
                items: p.list("items", |o| {
                    Ok(MenuItem {
                        id: o.required_text("id")?,
                        label: o.required_text("label")?,
                        icon: o.icon()?,
                        shortcut: o.optional_text("shortcut")?,
                        disabled: o.flag("disabled")?,
                        danger: o.flag("danger")?,
                        checked: o.optional_flag("checked")?,
                        separator_before: o.flag("separatorBefore")?,
                        ..MenuItem::default()
                    })
                })?,
            },
            _ => return Err(error(format!("Unknown GlassDesign widget: {kind:?}"))),
        })
    }

    pub fn uses_overlay(&self) -> bool {
        matches!(self, Self::Select { .. } | Self::Menu { .. })
    }

    pub fn state(&self) -> LiveProps {
        match self {
            Self::Segmented { value, .. } | Self::Select { value, .. } => {
                LiveProps::Value(RwSignal::new(value.clone()))
            }
            Self::SearchField {
                value, disabled, ..
            } => LiveProps::Search {
                value: RwSignal::new(value.clone()),
                disabled: RwSignal::new(*disabled),
            },
            Self::SplitGrip {
                value, min, max, ..
            } => LiveProps::Grip {
                value: RwSignal::new(*value),
                min: RwSignal::new(*min),
                max: RwSignal::new(*max),
            },
            Self::TrafficLights {
                focused,
                maximized,
                dirty,
                ..
            } => LiveProps::Window {
                focused: RwSignal::new(*focused),
                maximized: RwSignal::new(*maximized),
                dirty: RwSignal::new(*dirty),
            },
            _ => LiveProps::Static,
        }
    }

    pub fn render(self, emit: Emitter, has_right_slot: bool, live: LiveProps) -> AnyView {
        match self {
            Self::Button { label, icon, variant, size, disabled, class } => {
                let mut p = ButtonProps::builder().label(label).variant(variant).size(size)
                    .disabled(disabled).class(class).on_click(emit.action()).build();
                p.icon = icon;
                Button(p).into_any()
            }
            Self::IconButton { label, icon, variant, size, disabled, active, toggle, class } => view! {
                <IconButton label=label icon=icon variant=variant size=size disabled=disabled active=active toggle=toggle
                    class=class on_click=emit.action()/>
            }.into_any(),
            Self::NavItem { label, icon, selected, count, disabled } => {
                let mut p = NavItemProps::builder().label(label).selected(selected)
                    .disabled(disabled).on_click(emit.action()).build();
                p.icon = icon;
                p.count = count;
                NavItem(p).into_any()
            }
            Self::NavCard { title, desc, icon } => {
                let mut p = NavCardProps::builder().title(title).desc(desc).solid(true).on_click(emit.action()).build();
                p.icon = icon;
                NavCard(p).into_any()
            }
            Self::Segmented { label, options, .. } => {
                let LiveProps::Value(value) = live else { unreachable!() };
                let change = emit.change(value, true);
                view! { <Segmented label=label options=options value=value on_change=change/> }.into_any()
            }
            Self::SearchField { label, placeholder, width, id, .. } => {
                let LiveProps::Search { value, disabled } = live else { unreachable!() };
                let input = emit.search_input(value);
                view! {
                    <SearchField value=value label=label placeholder=placeholder width=width disabled=disabled id=id on_input=input/>
                }.into_any()
            }
            Self::Select { options, label, width, .. } => {
                let LiveProps::Value(value) = live else { unreachable!() };
                let change = emit.change(value, false);
                view! { <Select options=options value=value label=label width=width on_change=change/> }.into_any()
            }
            Self::InlineMessage { tone, title, message } => view! {
                <InlineMessage tone=tone title=title description=message/>
            }.into_any(),
            Self::Badge { label, variant, icon } => {
                let mut p = BadgeProps::builder().label(label).variant(variant).build();
                p.icon = icon;
                Badge(p).into_any()
            }
            Self::Card { class } => view! { <Card class=class children=Box::new(|| ().into_any())/> }.into_any(),
            Self::StatusBar => {
                let mut p = StatusBarProps::builder().children(Box::new(|| ().into_any())).build();
                p.right = has_right_slot.then(|| Box::new(|| ().into_any()) as Children);
                StatusBar(p).into_any()
            }
            Self::Toolbar => view! { <Toolbar children=Box::new(|| ().into_any())/> }.into_any(),
            Self::Capsule { size, class } => view! { <Capsule size=size class=class children=Box::new(|| ().into_any())/> }.into_any(),
            Self::GlassPanel { size } => {
                let mut p = glass_design::layout::glass_panel::GlassPanelProps::builder()
                    .children(Box::new(|| ().into_any())).build();
                p.size = size;
                GlassPanel(p).into_any()
            }
            Self::SplitGrip { label, controls, .. } => {
                let LiveProps::Grip { value, min, max } = live else { unreachable!() };
                let on_change = Callback::new(move |next: u32| {
                    value.set(next);
                    emit.queue(Some(next.to_string()));
                });
                view! {
                    <SplitGrip value=value.into() min=min.into() max=max.into()
                        label=label controls=controls on_change=on_change />
                }.into_any()
            }
            Self::TrafficLights { reveal, .. } => {
                let LiveProps::Window { focused, maximized, dirty } = live else { unreachable!() };
                let action = |name: &str| {
                    let emit = emit.clone();
                    let name = name.to_owned();
                    Callback::new(move |_| emit.queue(Some(name.clone())))
                };
                view! {
                    <TrafficLights focused=focused maximized=maximized dirty=dirty reveal=reveal
                        on_close=action("close") on_minimize=action("minimize") on_zoom=action("zoom") />
                }.into_any()
            }
            Self::Menu { label, icon, class, items } => {
                let on_select = Callback::new(move |id| emit.queue(Some(id)));
                let mut p = MenuButtonProps::builder().label(label).class(class).items(items)
                    .on_select(on_select).build();
                p.icon = icon;
                MenuButton(p).into_any()
            }
        }
    }
}

/// Reactive props share the original component owner. Updating one never remounts it.
#[derive(Clone, Copy)]
pub(crate) enum LiveProps {
    Static,
    Value(RwSignal<String>),
    Search {
        value: RwSignal<String>,
        disabled: RwSignal<bool>,
    },
    Grip {
        value: RwSignal<u32>,
        min: RwSignal<u32>,
        max: RwSignal<u32>,
    },
    Window {
        focused: RwSignal<bool>,
        maximized: RwSignal<bool>,
        dirty: RwSignal<bool>,
    },
}

fn set_if_changed<T: PartialEq + Send + Sync + 'static>(signal: RwSignal<T>, value: Option<T>) {
    if let Some(value) = value.filter(|v| signal.with_untracked(|current| current != v)) {
        signal.set(value);
    }
}

impl LiveProps {
    pub fn update(self, patch: &JsValue) -> Result<(), JsValue> {
        let (kind, keys): (&str, &[_]) = match self {
            Self::Static => ("static widget", &[]),
            Self::Value(_) => ("controlled widget", &["value"]),
            Self::Search { .. } => ("search-field", &["value", "disabled"]),
            Self::Grip { .. } => ("split-grip", &["value", "min", "max"]),
            Self::Window { .. } => ("traffic-lights", &["focused", "maximized", "dirty"]),
        };
        let p = Props::new(patch, kind)?;
        p.allow_only(keys)?;
        match self {
            Self::Static => {}
            Self::Value(value) => set_if_changed(value, p.optional_text("value")?),
            Self::Search { value, disabled } => {
                let next = (p.optional_text("value")?, p.optional_flag("disabled")?);
                set_if_changed(value, next.0);
                set_if_changed(disabled, next.1);
            }
            Self::Grip { value, min, max } => {
                let next = (p.uint("value")?, p.uint("min")?, p.uint("max")?);
                set_if_changed(value, next.0);
                set_if_changed(min, next.1);
                set_if_changed(max, next.2);
            }
            Self::Window {
                focused,
                maximized,
                dirty,
            } => {
                let next = (
                    p.optional_flag("focused")?,
                    p.optional_flag("maximized")?,
                    p.optional_flag("dirty")?,
                );
                set_if_changed(focused, next.0);
                set_if_changed(maximized, next.1);
                set_if_changed(dirty, next.2);
            }
        }
        Ok(())
    }
}
