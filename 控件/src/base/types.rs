use leptos::prelude::*;

/// 控件尺寸阶梯。`Md` 不写类，沿用旧库的默认值。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub fn class(self) -> Option<&'static str> {
        match self {
            Self::Sm => Some("sm"),
            Self::Md => None,
            Self::Lg => Some("lg"),
        }
    }

    pub fn icon_px(self) -> u16 {
        match self {
            Self::Sm => 13,
            Self::Md => 14,
            Self::Lg => 16,
        }
    }
}

/// 可选的响应式布尔值：`bool`、`Signal<bool>`、`MaybeProp<bool>` 都能传。
pub type BoolProp = MaybeProp<bool>;
