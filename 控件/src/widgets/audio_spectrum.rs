use leptos::prelude::*;

/// 音频频谱尺寸档位。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SpectrumSize {
    /// 紧凑型（适合嵌入底栏）
    Compact,
    /// 标准型（默认）
    #[default]
    Normal,
    /// 宽幅型（适合正在播放大视图）
    Large,
}

impl SpectrumSize {
    fn class(self) -> Option<&'static str> {
        match self {
            Self::Compact => Some("c-spectrum--compact"),
            Self::Normal => None,
            Self::Large => Some("c-spectrum--large"),
        }
    }
}

const DEFAULT_IDLE_BARS: usize = 16;
const MIN_BAR_LEVEL: f32 = 0.06;

pub(crate) fn normalize_bar_level(value: f32, active: bool) -> f32 {
    if !active || !value.is_finite() {
        return MIN_BAR_LEVEL;
    }
    value.clamp(MIN_BAR_LEVEL, 1.0)
}

/// 实时音频频谱柱状可视化组件。
///
/// 通过 CSS `transform: scaleY(var(--bar-level))` 平滑缩放频谱柱，
/// 仅在频谱柱数量变化时重建 DOM 节点，帧级刷新保持零抖动。
#[component]
pub fn AudioSpectrum(
    #[prop(into)] bars: Signal<Vec<f32>>,
    #[prop(into, optional)] active: MaybeProp<bool>,
    #[prop(optional)] size: SpectrumSize,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let mut cls = String::from("c-spectrum");
    if let Some(s) = size.class() {
        cls.push(' ');
        cls.push_str(s);
    }
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let is_active = move || active.get().unwrap_or(false);
    let bar_count = Memo::new(move |_| {
        let len = bars.with(|v| v.len());
        if len == 0 {
            DEFAULT_IDLE_BARS
        } else {
            len
        }
    });

    view! {
        <div
            class=cls
            role="img"
            aria-label="音频频谱"
            data-active=move || if is_active() { "true" } else { "false" }
        >
            {move || {
                let count = bar_count.get();
                (0..count)
                    .map(|idx| {
                        let style = move || {
                            let active_now = is_active();
                            let raw = bars.with(|v| v.get(idx).copied().unwrap_or(MIN_BAR_LEVEL));
                            let level = normalize_bar_level(raw, active_now);
                            format!("--bar-level:{level:.4}")
                        };
                        view! { <span class="c-spectrum__bar" style=style></span> }
                    })
                    .collect_view()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_bar_level, MIN_BAR_LEVEL};

    #[test]
    fn inactive_and_non_finite_collapse_to_baseline() {
        assert_eq!(normalize_bar_level(0.9, false), MIN_BAR_LEVEL);
        assert_eq!(normalize_bar_level(f32::NAN, true), MIN_BAR_LEVEL);
        assert_eq!(normalize_bar_level(f32::INFINITY, true), MIN_BAR_LEVEL);
    }

    #[test]
    fn active_values_clamp_into_range() {
        assert_eq!(normalize_bar_level(-0.5, true), MIN_BAR_LEVEL);
        assert_eq!(normalize_bar_level(0.42, true), 0.42);
        assert_eq!(normalize_bar_level(1.8, true), 1.0);
    }
}
