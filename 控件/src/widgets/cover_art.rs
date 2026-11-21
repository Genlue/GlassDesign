use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::widgets::media_icons::media_icons;

/// 专辑/歌单封面尺寸档位。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum CoverArtSize {
    /// 32 × 32
    Xs,
    /// 40 × 40
    Sm,
    /// 56 × 56（默认）
    #[default]
    Md,
    /// 120 × 120
    Lg,
    /// 220 × 220
    Hero,
}

impl CoverArtSize {
    fn class(self) -> &'static str {
        match self {
            Self::Xs => "c-cover--xs",
            Self::Sm => "c-cover--sm",
            Self::Md => "c-cover--md",
            Self::Lg => "c-cover--lg",
            Self::Hero => "c-cover--hero",
        }
    }

    fn note_icon_px(self) -> u16 {
        match self {
            Self::Xs => 14,
            Self::Sm => 18,
            Self::Md => 22,
            Self::Lg => 36,
            Self::Hero => 52,
        }
    }

    fn play_icon_px(self) -> u16 {
        match self {
            Self::Xs => 12,
            Self::Sm => 14,
            Self::Md => 18,
            Self::Lg => 22,
            Self::Hero => 26,
        }
    }
}

/// 专辑/曲目封面组件。
///
/// 支持空状态玻璃音符占位、加载失败自动回退、黑胶旋转模式（`spinning`）、
/// 正在播放状态指示（`playing`）以及悬停播放按钮遮罩（`on_play`）。
#[component]
pub fn CoverArt(
    #[prop(into, optional)] src: MaybeProp<String>,
    #[prop(optional)] size: CoverArtSize,
    #[prop(into, optional)] alt: String,
    #[prop(into, optional)] spinning: MaybeProp<bool>,
    #[prop(into, optional)] playing: MaybeProp<bool>,
    #[prop(optional)] on_play: Option<Callback<()>>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let loaded_src = RwSignal::new(Option::<String>::None);
    let failed_src = RwSignal::new(Option::<String>::None);

    let mut cls = format!("c-cover {}", size.class());
    if on_play.is_some() {
        cls.push_str(" c-cover--interactive");
    }
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    let is_spinning = move || spinning.get().unwrap_or(false);
    let is_playing = move || playing.get().unwrap_or(false);
    let note_px = size.note_icon_px();
    let play_px = size.play_icon_px();

    let resolved_src = Memo::new(move |_| {
        src.get()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    });

    view! {
        <div
            class=cls
            data-spinning=move || is_spinning().then_some("true")
            data-playing=move || is_playing().then_some("true")
        >
            {move || {
                let current = resolved_src.get();
                let failed = failed_src.get();
                match current {
                    Some(url) if failed.as_deref() != Some(url.as_str()) => {
                        let url_load = url.clone();
                        let url_err = url.clone();
                        let url_check = url.clone();
                        let alt_text = alt.clone();
                        view! {
                            <img
                                class="c-cover__img"
                                src=url
                                alt=alt_text
                                draggable="false"
                                data-loaded=move || {
                                    (loaded_src.get().as_deref() == Some(url_check.as_str())).then_some("1")
                                }
                                on:load=move |_| loaded_src.set(Some(url_load.clone()))
                                on:error=move |_| failed_src.set(Some(url_err.clone()))
                            />
                        }
                        .into_any()
                    }
                    _ => {
                        let placeholder_icon = if is_spinning() {
                            media_icons::DISC
                        } else {
                            media_icons::MUSIC
                        };
                        view! {
                            <div class="c-cover__placeholder" aria-hidden="true">
                                <Icon name=placeholder_icon size=note_px />
                            </div>
                        }
                        .into_any()
                    }
                }
            }}

            {move || {
                is_spinning().then(|| view! { <span class="c-cover__spindle" aria-hidden="true"></span> })
            }}

            {move || {
                (is_playing() && size != CoverArtSize::Xs).then(|| view! {
                    <span class="c-cover__eq" aria-hidden="true">
                        <span></span>
                        <span></span>
                        <span></span>
                    </span>
                })
            }}

            {on_play.map(|cb| {
                view! {
                    <button
                        type="button"
                        class="c-cover__play"
                        aria-label=move || if is_playing() { "暂停" } else { "播放" }
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(());
                        }
                    >
                        {move || {
                            let icon = if is_playing() { IconName::PAUSE } else { IconName::PLAY };
                            view! { <Icon name=icon size=play_px /> }
                        }}
                    </button>
                }
            })}
        </div>
    }
}
