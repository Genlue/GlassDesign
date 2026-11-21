//! 媒体与音乐播放器专用图标常量。
//!
//! 通过 `IconName::custom(...)` 定义，避免修改由 `scripts/gen-icons.mjs`
//! 生成的 `base/icon.rs`。全部采用 24×24 视框，跟随 CSS `--ic-stroke`。

use crate::base::icon::IconName;

pub mod media_icons {
    use super::IconName;

    /// 音符（封面占位 / 曲目空状态）
    pub const MUSIC: IconName = IconName::custom(
        r#"<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>"#,
    );

    /// 黑胶唱片
    pub const DISC: IconName = IconName::custom(
        r#"<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="2.5"/><path d="M6.8 6.8a7.5 7.5 0 0 1 10.4 0"/>"#,
    );

    /// 上一曲
    pub const SKIP_BACK: IconName = IconName::custom(
        r#"<polygon points="19 20 9 12 19 4 19 20"/><line x1="5" x2="5" y1="19" y2="5"/>"#,
    );

    /// 下一曲
    pub const SKIP_FORWARD: IconName = IconName::custom(
        r#"<polygon points="5 4 15 12 5 20 5 4"/><line x1="19" x2="19" y1="5" y2="19"/>"#,
    );

    /// 顺序播放
    pub const SEQUENTIAL: IconName = IconName::custom(
        r#"<path d="M3 6h15"/><path d="m15 3 3 3-3 3"/><path d="M3 12h12"/><path d="M3 18h9"/>"#,
    );

    /// 列表循环
    pub const REPEAT: IconName = IconName::custom(
        r#"<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>"#,
    );

    /// 单曲循环
    pub const REPEAT_1: IconName = IconName::custom(
        r#"<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/><path d="M11 10h1v4"/>"#,
    );

    /// 播放队列
    pub const LIST_MUSIC: IconName = IconName::custom(
        r#"<path d="M21 15V6"/><path d="M18.5 18a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Z"/><path d="M12 12H3"/><path d="M16 6H3"/><path d="M12 18H3"/>"#,
    );

    /// 歌词视图
    pub const LYRICS: IconName = IconName::custom(
        r#"<path d="M4 6h16"/><path d="M4 12h11"/><path d="M4 18h8"/><circle cx="18.5" cy="17.5" r="2"/><path d="M20.5 17.5V11"/>"#,
    );

    /// 实心喜欢（红心）
    pub const HEART_FILLED: IconName = IconName::custom(
        r#"<path fill="currentColor" d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>"#,
    );

    /// 低音量
    pub const VOLUME_1: IconName = IconName::custom(
        r#"<path d="M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z"/><path d="M16 9a5 5 0 0 1 0 6"/>"#,
    );
}
