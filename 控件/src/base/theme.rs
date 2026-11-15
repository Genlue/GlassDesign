use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 主题模式。`System` 跟随 `prefers-color-scheme`。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }
}

/// 全局主题。只写 CSS 变量与根属性，不产生任何样式规则。
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub mode: RwSignal<ThemeMode>,
    pub accent: RwSignal<&'static str>,
    pub glass: RwSignal<f32>,
    pub shadow: RwSignal<f32>,
    pub stroke: RwSignal<f32>,
}

impl Theme {
    /// 在应用根节点调用一次；之后用 `use_context::<Theme>()` 取。
    pub fn provide() -> Self {
        let theme = Self {
            mode: RwSignal::new(ThemeMode::System),
            accent: RwSignal::new("#0a84ff"),
            glass: RwSignal::new(1.0),
            shadow: RwSignal::new(1.0),
            stroke: RwSignal::new(1.4),
        };
        provide_context(theme);
        Effect::new(move |_| theme.apply());
        theme
    }

    pub fn set_mode(&self, mode: ThemeMode) {
        self.mode.set(mode);
    }

    pub fn set_accent(&self, hex: &'static str) {
        self.accent.set(hex);
    }

    fn apply(&self) {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(root) = doc.document_element() else {
            return;
        };
        let resolved = match self.mode.get() {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => {
                if prefers_dark() {
                    "dark"
                } else {
                    "light"
                }
            }
        };

        let _ = root.set_attribute("data-theme", resolved);
        let _ = root.set_attribute("data-theme-mode", self.mode.get().as_str());
        let _ = root.set_attribute("data-gd-version", crate::base::install::VERSION);

        let Ok(html) = root.dyn_into::<web_sys::HtmlElement>() else {
            return;
        };
        let style = html.style();
        let _ = style.set_property("--lg", &self.glass.get().to_string());
        let _ = style.set_property("--shadow-k", &self.shadow.get().to_string());
        let _ = style.set_property("--ic-stroke", &self.stroke.get().to_string());
        apply_accent(&style, self.accent.get(), resolved);
    }
}

fn prefers_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .map(|mq| mq.matches())
        .unwrap_or(false)
}

fn hex_rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim().trim_start_matches('#');
    let full = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        h.to_string()
    };
    let n = u32::from_str_radix(&full, 16).unwrap_or(0x0a84ff);
    (((n >> 16) & 255) as u8, ((n >> 8) & 255) as u8, (n & 255) as u8)
}

fn mix((r, g, b): (u8, u8, u8), (r2, g2, b2): (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let f = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    (f(r, r2), f(g, g2), f(b, b2))
}

fn rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("rgb({r} {g} {b})")
}

fn rgba((r, g, b): (u8, u8, u8), a: f32) -> String {
    format!("rgb({r} {g} {b} / {a})")
}

fn luminance((r, g, b): (u8, u8, u8)) -> f32 {
    let f = |v: u8| {
        let s = v as f32 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}

fn apply_accent(style: &web_sys::CssStyleDeclaration, hex: &str, resolved: &str) {
    let base = hex_rgb(hex);
    let adjusted = if resolved == "dark" {
        mix(base, (255, 255, 255), 0.06)
    } else {
        mix(base, (0, 0, 0), 0.10)
    };
    let hover = if resolved == "dark" {
        mix(base, (255, 255, 255), 0.16)
    } else {
        mix(base, (0, 0, 0), 0.22)
    };

    let _ = style.set_property("--accent", &rgb(adjusted));
    let _ = style.set_property("--accent-hover", &rgb(hover));
    let _ = style.set_property("--accent-soft", &rgba(base, if resolved == "dark" { 0.22 } else { 0.12 }));
    let _ = style.set_property("--accent-ring", &rgba(base, if resolved == "dark" { 0.48 } else { 0.36 }));
    let _ = style.set_property("--accent-rgb", &format!("{} {} {}", base.0, base.1, base.2));
    let _ = style.set_property(
        "--text-on-accent",
        if luminance(base) > 0.62 { "#1d1d1f" } else { "#ffffff" },
    );
}
