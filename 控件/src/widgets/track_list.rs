use std::collections::HashSet;

use leptos::prelude::*;

use crate::base::icon::{Icon, IconName};
use crate::widgets::cover_art::{CoverArt, CoverArtSize};
use crate::widgets::media_icons::media_icons;

/// 曲目列表条目数据。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TrackItem {
    pub id: String,
    /// 展示序号（建议传入从 1 开始的序号）。
    pub index: usize,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub cover_url: Option<String>,
    pub source_label: Option<String>,
    pub quality_label: Option<String>,
    pub liked: bool,
}

/// 音乐曲目列表组件。
///
/// 接收结构化 `TrackItem` 列表，支持当前播放行高亮与跳动均衡器指示、
/// 封面/序号悬停播放按钮、来源与音质徽标、喜欢切换、加入队列与更多操作。
#[component]
pub fn TrackList(
    #[prop(into)] items: Signal<Vec<TrackItem>>,
    #[prop(into, optional)] active_id: MaybeProp<String>,
    #[prop(into, optional)] is_playing: MaybeProp<bool>,
    #[prop(optional)] on_play: Option<Callback<String>>,
    #[prop(optional)] on_like: Option<Callback<String>>,
    #[prop(optional)] on_add_queue: Option<Callback<String>>,
    #[prop(optional)] on_more: Option<Callback<String>>,
    #[prop(optional)] on_context_menu: Option<Callback<(String, f64, f64)>>,
    #[prop(into, optional)] selectable: MaybeProp<bool>,
    #[prop(into, optional)] selected_ids: MaybeProp<HashSet<String>>,
    #[prop(optional)] on_toggle_select: Option<Callback<(String, bool)>>,
    #[prop(into, optional)] empty_text: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let mut base_cls = String::from("c-tracklist");
    if !class.is_empty() {
        base_cls.push(' ');
        base_cls.push_str(&class);
    }

    let empty_msg = if empty_text.is_empty() {
        String::from("暂无曲目")
    } else {
        empty_text
    };

    let is_selectable = move || selectable.get().unwrap_or(false);
    let selected_set = Memo::new(move |_| selected_ids.get().unwrap_or_default());

    view! {
        <div class=move || {
            if is_selectable() {
                format!("{base_cls} c-tracklist--selectable")
            } else {
                base_cls.clone()
            }
        }>
            {move || {
                let list = items.get();
                if list.is_empty() {
                    view! {
                        <div class="c-tracklist__empty">
                            <span class="c-tracklist__empty-icon">
                                <Icon name=media_icons::MUSIC size=28 />
                            </span>
                            <span>{empty_msg.clone()}</span>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div
                            class="c-tracklist__rows"
                            role="listbox"
                            aria-label="曲目列表"
                            aria-multiselectable=move || is_selectable().then_some("true")
                        >
                            <For
                                each=move || items.get()
                                key=|item| {
                                    (
                                        item.id.clone(),
                                        item.index,
                                        item.liked,
                                        item.title.clone(),
                                        item.artist.clone(),
                                        item.album.clone(),
                                        item.duration.clone(),
                                        item.cover_url.clone(),
                                        item.source_label.clone(),
                                        item.quality_label.clone(),
                                    )
                                }
                                children=move |item| {
                                    let id_active = item.id.clone();
                                    let id_playing = item.id.clone();
                                    let id_aria = item.id.clone();
                                    let id_sel = item.id.clone();
                                    let id_sel_data = item.id.clone();
                                    let id_sel_check = item.id.clone();
                                    let id_eq = item.id.clone();
                                    let id_btn_icon = item.id.clone();
                                    let id_play_row = item.id.clone();
                                    let id_sel_row = item.id.clone();
                                    let id_play_btn = item.id.clone();
                                    let id_key = item.id.clone();
                                    let id_like = item.id.clone();
                                    let id_queue = item.id.clone();
                                    let id_more = item.id.clone();
                                    let id_ctx = item.id.clone();

                                    let is_row_active = move || {
                                        active_id.get().as_deref() == Some(id_active.as_str())
                                    };
                                    let is_row_playing = move || {
                                        is_playing.get().unwrap_or(false)
                                            && active_id.get().as_deref() == Some(id_playing.as_str())
                                    };
                                    let is_row_selected = move || {
                                        is_selectable() && selected_set.with(|s| s.contains(&id_sel))
                                    };

                                    let index_text = format!("{:02}", item.index);
                                    let title = item.title.clone();
                                    let artist = item.artist.clone();
                                    let album = item.album.clone();
                                    let duration = item.duration.clone();
                                    let cover_url = item.cover_url.clone();
                                    let source_label = item.source_label.clone();
                                    let quality_label = item.quality_label.clone();
                                    let liked = item.liked;
                                    let is_interactive = on_play.is_some();

                                    view! {
                                        <div
                                            class=move || if is_interactive || is_selectable() {
                                                "c-tracklist__row c-tracklist__row--interactive"
                                            } else {
                                                "c-tracklist__row"
                                            }
                                            role="option"
                                            tabindex="0"
                                            aria-selected=move || {
                                                if is_selectable() {
                                                    if selected_set.with(|s| s.contains(&id_sel_data)) {
                                                        "true"
                                                    } else {
                                                        "false"
                                                    }
                                                } else if active_id.get().as_deref() == Some(id_aria.as_str()) {
                                                    "true"
                                                } else {
                                                    "false"
                                                }
                                            }
                                            data-active=move || is_row_active().then_some("true")
                                            data-playing=move || is_row_playing().then_some("true")
                                            data-selected=move || is_row_selected().then_some("true")
                                            on:click=move |ev: web_sys::MouseEvent| {
                                                if is_selectable() {
                                                    if let Some(cb) = on_toggle_select {
                                                        cb.run((id_sel_row.clone(), ev.shift_key()));
                                                    }
                                                } else if let Some(cb) = on_play {
                                                    cb.run(id_play_row.clone());
                                                }
                                            }
                                            on:contextmenu=move |ev: web_sys::MouseEvent| {
                                                if let Some(cb) = on_context_menu {
                                                    ev.prevent_default();
                                                    ev.stop_propagation();
                                                    cb.run((
                                                        id_ctx.clone(),
                                                        ev.client_x() as f64,
                                                        ev.client_y() as f64,
                                                    ));
                                                }
                                            }
                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                let k = ev.key();
                                                if k == "Enter" || k == " " {
                                                    ev.prevent_default();
                                                    if is_selectable() {
                                                        if let Some(cb) = on_toggle_select {
                                                            cb.run((id_key.clone(), ev.shift_key()));
                                                        }
                                                    } else if let Some(cb) = on_play {
                                                        cb.run(id_key.clone());
                                                    }
                                                }
                                            }
                                        >
                                            {move || is_selectable().then(|| {
                                                let id_cb = id_sel_check.clone();
                                                let id_chk1 = id_sel_check.clone();
                                                let id_chk2 = id_sel_check.clone();
                                                let id_chk3 = id_sel_check.clone();
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="c-tracklist__check-cell"
                                                        role="checkbox"
                                                        aria-checked=move || {
                                                            if selected_set.with(|s| s.contains(&id_chk1)) {
                                                                "true"
                                                            } else {
                                                                "false"
                                                            }
                                                        }
                                                        aria-label="选择曲目"
                                                        on:click=move |ev: web_sys::MouseEvent| {
                                                            ev.stop_propagation();
                                                            if let Some(cb) = on_toggle_select {
                                                                cb.run((id_cb.clone(), ev.shift_key()));
                                                            }
                                                        }
                                                    >
                                                        <span
                                                            class="c-check"
                                                            aria-hidden="true"
                                                            aria-checked=move || {
                                                                if selected_set.with(|s| s.contains(&id_chk2)) {
                                                                    "true"
                                                                } else {
                                                                    "false"
                                                                }
                                                            }
                                                        >
                                                            {move || {
                                                                selected_set
                                                                    .with(|s| s.contains(&id_chk3))
                                                                    .then(|| view! { <Icon name=IconName::CHECK size=11 /> })
                                                            }}
                                                        </span>
                                                    </button>
                                                }
                                            })}

                                            <div class="c-tracklist__leading">
                                                <span class="c-tracklist__num">
                                                    {move || {
                                                        let active_eq = active_id.get().as_deref() == Some(id_eq.as_str());
                                                        let playing_now = active_eq && is_playing.get().unwrap_or(false);
                                                        if playing_now {
                                                            view! {
                                                                <span class="c-tracklist__eq" aria-hidden="true">
                                                                    <span></span>
                                                                    <span></span>
                                                                    <span></span>
                                                                </span>
                                                            }
                                                            .into_any()
                                                        } else {
                                                            view! { <span>{index_text.clone()}</span> }.into_any()
                                                        }
                                                    }}
                                                </span>
                                                {on_play.map(|cb| {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="c-tracklist__playbtn"
                                                            aria-label="播放曲目"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                cb.run(id_play_btn.clone());
                                                            }
                                                        >
                                                            {move || {
                                                                let playing_now = is_playing.get().unwrap_or(false)
                                                                    && active_id.get().as_deref() == Some(id_btn_icon.as_str());
                                                                let icon = if playing_now { IconName::PAUSE } else { IconName::PLAY };
                                                                view! { <Icon name=icon size=14 /> }
                                                            }}
                                                        </button>
                                                    }
                                                })}
                                            </div>

                                            <div class="c-tracklist__main">
                                                <CoverArt
                                                    src=cover_url
                                                    size=CoverArtSize::Sm
                                                    alt=title.clone()
                                                    class="c-tracklist__cover"
                                                />
                                                <div class="c-tracklist__meta">
                                                    <div class="c-tracklist__title-row">
                                                        <span class="c-tracklist__title">{title}</span>
                                                        {quality_label.filter(|s| !s.is_empty()).map(|q| {
                                                            view! { <span class="c-badge c-badge--accent c-tracklist__badge">{q}</span> }
                                                        })}
                                                        {source_label.filter(|s| !s.is_empty()).map(|src| {
                                                            view! { <span class="c-badge c-tracklist__badge">{src}</span> }
                                                        })}
                                                    </div>
                                                    <div class="c-tracklist__artist">{artist}</div>
                                                </div>
                                            </div>

                                            <div class="c-tracklist__album">{album}</div>

                                            <div class="c-tracklist__actions">
                                                {on_like.map(|cb| {
                                                    let heart_icon = if liked {
                                                        media_icons::HEART_FILLED
                                                    } else {
                                                        IconName::HEART
                                                    };
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class=if liked {
                                                                "c-iconbtn c-tracklist__like c-tracklist__like--on"
                                                            } else {
                                                                "c-iconbtn c-tracklist__like"
                                                            }
                                                            aria-label=if liked { "取消喜欢" } else { "喜欢" }
                                                            aria-pressed=liked.to_string()
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                cb.run(id_like.clone());
                                                            }
                                                        >
                                                            <Icon name=heart_icon size=15 />
                                                        </button>
                                                    }
                                                })}
                                                {on_add_queue.map(|cb| {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="c-iconbtn c-tracklist__action"
                                                            aria-label="添加到播放队列"
                                                            title="添加到播放队列"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                cb.run(id_queue.clone());
                                                            }
                                                        >
                                                            <Icon name=media_icons::LIST_MUSIC size=15 />
                                                        </button>
                                                    }
                                                })}
                                                {on_more.map(|cb| {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="c-iconbtn c-tracklist__action"
                                                            aria-label="更多操作"
                                                            title="更多操作"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                cb.run(id_more.clone());
                                                            }
                                                        >
                                                            <Icon name=IconName::MORE size=15 />
                                                        </button>
                                                    }
                                                })}
                                            </div>

                                            <div class="c-tracklist__duration">{duration}</div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}
