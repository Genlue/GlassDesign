use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::base::types::Size;
use crate::widgets::audio_spectrum::{AudioSpectrum, SpectrumSize};
use crate::widgets::button::IconButton;
use crate::widgets::cover_art::{CoverArt, CoverArtSize};
use crate::widgets::media_icons::media_icons;
use crate::widgets::progress::Spinner;
use crate::widgets::slider::Slider;

/// 播放顺序模式。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum PlayMode {
    /// 顺序播放
    #[default]
    Sequential,
    /// 列表循环
    LoopList,
    /// 单曲循环
    LoopOne,
    /// 随机播放
    Shuffle,
}

impl PlayMode {
    /// 返回下一档播放模式，便于调用方直接用于 `on_cycle_mode`。
    pub fn next(self) -> Self {
        match self {
            Self::Sequential => Self::LoopList,
            Self::LoopList => Self::LoopOne,
            Self::LoopOne => Self::Shuffle,
            Self::Shuffle => Self::Sequential,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sequential => "顺序播放",
            Self::LoopList => "列表循环",
            Self::LoopOne => "单曲循环",
            Self::Shuffle => "随机播放",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Sequential => media_icons::SEQUENTIAL,
            Self::LoopList => media_icons::REPEAT,
            Self::LoopOne => media_icons::REPEAT_1,
            Self::Shuffle => IconName::SHUFFLE,
        }
    }
}

/// 底栏播放器展示所需的当前曲目摘要。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PlayerTrackInfo {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub cover_url: Option<String>,
    pub quality_label: Option<String>,
    pub source_label: Option<String>,
    pub liked: bool,
}

pub(crate) fn format_duration_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

/// 浮动液态玻璃底部播放控制栏。
///
/// 采用 GlassDesign 面板玻璃光学管线（`.c-glass-panel`），分为左（封面与曲目信息）、
/// 中（播控按钮组与可拖拽进度条）、右（实时频谱、音质徽标、音量与队列开关）三区。
#[component]
pub fn PlayerBar(
    #[prop(into)] track: Signal<Option<PlayerTrackInfo>>,
    #[prop(into)] playing: Signal<bool>,
    #[prop(into, optional)] loading: MaybeProp<bool>,
    #[prop(into)] current_ms: Signal<u64>,
    #[prop(into)] duration_ms: Signal<u64>,
    #[prop(into)] volume: Signal<f64>,
    #[prop(into)] muted: Signal<bool>,
    #[prop(into)] play_mode: Signal<PlayMode>,
    #[prop(into, optional)] lyrics_open: MaybeProp<bool>,
    #[prop(into, optional)] queue_open: MaybeProp<bool>,
    #[prop(optional)] spectrum: Option<Signal<Vec<f32>>>,
    #[prop(optional)] on_toggle_play: Option<Callback<()>>,
    #[prop(optional)] on_prev: Option<Callback<()>>,
    #[prop(optional)] on_next: Option<Callback<()>>,
    #[prop(optional)] on_seek: Option<Callback<u64>>,
    #[prop(optional)] on_volume: Option<Callback<f64>>,
    #[prop(optional)] on_toggle_mute: Option<Callback<()>>,
    #[prop(optional)] on_cycle_mode: Option<Callback<()>>,
    #[prop(optional)] on_toggle_like: Option<Callback<()>>,
    #[prop(optional)] on_toggle_lyrics: Option<Callback<()>>,
    #[prop(optional)] on_toggle_queue: Option<Callback<()>>,
    #[prop(optional)] on_open_now_playing: Option<Callback<()>>,
    #[prop(optional)] on_open_quality: Option<Callback<()>>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let mut cls = String::from("c-playerbar c-glass-panel");
    if !class.is_empty() {
        cls.push(' ');
        cls.push_str(&class);
    }

    // 拖拽进度条期间用本地预览时间，避免播放定时器每帧覆写拇指位置
    let scrubbing_ms = RwSignal::new(Option::<u64>::None);

    // 切歌时立刻清除残留的拖拽预览，避免新歌初始进度条停留在旧歌拖拽位置
    let track_id = Memo::new(move |_| track.with(|t| t.as_ref().map(|x| x.id.clone())));
    Effect::new(move |_| {
        let _ = track_id.get();
        scrubbing_ms.set(None);
    });

    let has_track = move || track.with(|t| t.is_some());
    let is_loading = move || loading.get().unwrap_or(false);
    let is_lyrics_open = move || lyrics_open.get().unwrap_or(false);
    let is_queue_open = move || queue_open.get().unwrap_or(false);

    let safe_duration = Signal::derive(move || duration_ms.get().max(1));
    let seek_max_ms = Signal::derive(move || safe_duration.get() as f64);
    let seek_disabled = Signal::derive(move || !has_track() || duration_ms.get() == 0);
    let shown_ms = Signal::derive(move || {
        let dur = duration_ms.get();
        match scrubbing_ms.get() {
            Some(ms) => {
                if dur > 0 {
                    ms.min(dur)
                } else {
                    ms
                }
            }
            None => {
                if dur > 0 {
                    current_ms.get().min(dur)
                } else {
                    0
                }
            }
        }
    });
    let seek_slider_val = Signal::derive(move || shown_ms.get() as f64);

    let effective_volume = Signal::derive(move || {
        if muted.get() {
            0.0
        } else {
            volume.get().clamp(0.0, 1.0)
        }
    });

    let cover_src = Signal::derive(move || {
        track
            .get()
            .and_then(|t| t.cover_url)
            .unwrap_or_default()
    });
    let cover_alt = move || {
        track
            .get()
            .map(|t| t.title)
            .unwrap_or_else(|| String::from("专辑封面"))
    };

    view! {
        <footer class=cls aria-label="播放控制栏">
            <div class="c-playerbar__left">
                <button
                    type="button"
                    class="c-playerbar__trackbtn"
                    disabled=move || !has_track()
                    aria-label="打开正在播放"
                    on:click=move |_| {
                        if has_track() {
                            if let Some(cb) = on_open_now_playing {
                                cb.run(());
                            }
                        }
                    }
                >
                    <CoverArt
                        src=cover_src
                        size=CoverArtSize::Sm
                        alt=cover_alt()
                        playing=playing
                        class="c-playerbar__cover"
                    />
                    <div class="c-playerbar__meta">
                        {move || match track.get() {
                            Some(t) => view! {
                                <div class="c-playerbar__title-row">
                                    <span class="c-playerbar__title">{t.title}</span>
                                    {t.source_label.filter(|s| !s.is_empty()).map(|s| {
                                        view! { <span class="c-badge c-playerbar__source">{s}</span> }
                                    })}
                                </div>
                                <div class="c-playerbar__artist">{t.artist}</div>
                            }
                            .into_any(),
                            None => view! {
                                <div class="c-playerbar__title-row">
                                    <span class="c-playerbar__title c-playerbar__title--idle">"未在播放"</span>
                                </div>
                                <div class="c-playerbar__artist">"选择一首歌曲开始聆听"</div>
                            }
                            .into_any(),
                        }}
                    </div>
                </button>

                {move || {
                    let liked = track.with(|t| t.as_ref().is_some_and(|x| x.liked));
                    let disabled = !has_track();
                    let icon = if liked { media_icons::HEART_FILLED } else { IconName::HEART };
                    let like_cls = if liked {
                        "c-iconbtn c-playerbar__like c-playerbar__like--on"
                    } else {
                        "c-iconbtn c-playerbar__like"
                    };
                    view! {
                        <button
                            type="button"
                            class=like_cls
                            aria-label=if liked { "取消喜欢" } else { "喜欢" }
                            aria-pressed=liked.to_string()
                            disabled=disabled
                            on:click=move |_| {
                                if let Some(cb) = on_toggle_like {
                                    cb.run(());
                                }
                            }
                        >
                            <Icon name=icon size=16 />
                        </button>
                    }
                }}
            </div>

            <div class="c-playerbar__center">
                <div class="c-playerbar__controls">
                    <button
                        type="button"
                        class="c-iconbtn c-iconbtn--sm c-playerbar__mode"
                        aria-label=move || play_mode.get().label()
                        title=move || play_mode.get().label()
                        on:click=move |_| {
                            if let Some(cb) = on_cycle_mode {
                                cb.run(());
                            }
                        }
                    >
                        {move || view! { <Icon name=play_mode.get().icon() size=14 /> }}
                    </button>

                    <IconButton
                        icon=media_icons::SKIP_BACK
                        label="上一曲"
                        size=Size::Md
                        disabled=false
                        on_click=Callback::new(move |_| {
                            if let Some(cb) = on_prev {
                                cb.run(());
                            }
                        })
                    />

                    <button
                        type="button"
                        class="c-playerbar__playbtn"
                        aria-label=move || if playing.get() { "暂停" } else { "播放" }
                        disabled=move || !has_track() && !playing.get()
                        on:click=move |_| {
                            if let Some(cb) = on_toggle_play {
                                cb.run(());
                            }
                        }
                    >
                        {move || {
                            if is_loading() {
                                view! { <Spinner label="缓冲中" /> }.into_any()
                            } else {
                                let icon = if playing.get() { IconName::PAUSE } else { IconName::PLAY };
                                view! { <Icon name=icon size=18 /> }.into_any()
                            }
                        }}
                    </button>

                    <IconButton
                        icon=media_icons::SKIP_FORWARD
                        label="下一曲"
                        size=Size::Md
                        disabled=false
                        on_click=Callback::new(move |_| {
                            if let Some(cb) = on_next {
                                cb.run(());
                            }
                        })
                    />

                    <button
                        type="button"
                        class="c-iconbtn c-iconbtn--sm"
                        aria-label="歌词"
                        aria-pressed=move || is_lyrics_open().to_string()
                        title="歌词"
                        on:click=move |_| {
                            if let Some(cb) = on_toggle_lyrics {
                                cb.run(());
                            }
                        }
                    >
                        <Icon name=media_icons::LYRICS size=14 />
                    </button>
                </div>

                <div class="c-playerbar__progress">
                    <span class="c-playerbar__time">{move || format_duration_ms(shown_ms.get())}</span>
                    <div class="c-playerbar__seek">
                        <Slider
                            value=seek_slider_val
                            min=0.0
                            max=seek_max_ms
                            step=100.0
                            label="播放进度"
                            disabled=seek_disabled
                            on_input=Callback::new(move |val: f64| {
                                scrubbing_ms.set(Some(val.max(0.0).round() as u64));
                            })
                            on_change=Callback::new(move |val: f64| {
                                let target = val.max(0.0).round() as u64;
                                scrubbing_ms.set(None);
                                if let Some(cb) = on_seek {
                                    cb.run(target);
                                }
                            })
                            on_cancel=Callback::new(move |_| {
                                scrubbing_ms.set(None);
                            })
                        />
                    </div>
                    <span class="c-playerbar__time">{move || format_duration_ms(duration_ms.get())}</span>
                </div>
            </div>

            <div class="c-playerbar__right">
                {spectrum.map(|bars| {
                    view! {
                        <AudioSpectrum
                            bars=bars
                            active=playing
                            size=SpectrumSize::Compact
                            class="c-playerbar__spectrum"
                        />
                    }
                })}

                {move || {
                    track
                        .get()
                        .and_then(|t| t.quality_label)
                        .filter(|q| !q.is_empty())
                        .map(|q| {
                            view! {
                                <button
                                    type="button"
                                    class="c-playerbar__quality"
                                    aria-label="切换音质"
                                    on:click=move |_| {
                                        if let Some(cb) = on_open_quality {
                                            cb.run(());
                                        }
                                    }
                                >
                                    {q}
                                </button>
                            }
                        })
                }}

                <div class="c-playerbar__volume">
                    <button
                        type="button"
                        class="c-iconbtn c-iconbtn--sm"
                        aria-label=move || if muted.get() { "取消静音" } else { "静音" }
                        on:click=move |_| {
                            if let Some(cb) = on_toggle_mute {
                                cb.run(());
                            }
                        }
                    >
                        {move || {
                            let icon = if muted.get() || volume.get() <= 0.001 {
                                IconName::VOLUME_X
                            } else if volume.get() < 0.5 {
                                media_icons::VOLUME_1
                            } else {
                                IconName::VOLUME_2
                            };
                            view! { <Icon name=icon size=15 /> }
                        }}
                    </button>
                    <div class="c-playerbar__vol-slider">
                        <Slider
                            value=effective_volume
                            min=0.0
                            max=1.0
                            step=0.01
                            label="音量"
                            on_input=Callback::new(move |val: f64| {
                                if let Some(cb) = on_volume {
                                    cb.run(val.clamp(0.0, 1.0));
                                }
                            })
                            on_change=Callback::new(move |val: f64| {
                                if let Some(cb) = on_volume {
                                    cb.run(val.clamp(0.0, 1.0));
                                }
                            })
                        />
                    </div>
                </div>

                <button
                    type="button"
                    class="c-iconbtn c-iconbtn--sm c-playerbar__queuebtn"
                    aria-label="播放队列"
                    aria-pressed=move || is_queue_open().to_string()
                    title="播放队列"
                    on:click=move |_| {
                        if let Some(cb) = on_toggle_queue {
                            cb.run(());
                        }
                    }
                >
                    <Icon name=media_icons::LIST_MUSIC size=15 />
                </button>
            </div>
        </footer>
    }
}

#[cfg(test)]
mod tests {
    use super::{format_duration_ms, PlayMode};

    #[test]
    fn play_mode_cycles_in_order() {
        let m = PlayMode::Sequential;
        assert_eq!(m.next(), PlayMode::LoopList);
        assert_eq!(m.next().next(), PlayMode::LoopOne);
        assert_eq!(m.next().next().next(), PlayMode::Shuffle);
        assert_eq!(m.next().next().next().next(), PlayMode::Sequential);
    }

    #[test]
    fn duration_formats_mm_ss_and_hh_mm_ss() {
        assert_eq!(format_duration_ms(0), "00:00");
        assert_eq!(format_duration_ms(45_000), "00:45");
        assert_eq!(format_duration_ms(215_900), "03:35");
        assert_eq!(format_duration_ms(3_661_000), "1:01:01");
    }
}
