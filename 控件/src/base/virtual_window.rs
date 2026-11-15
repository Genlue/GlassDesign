//! 固定行高虚拟窗口：只回答两个问题 —— 「该渲染哪几行」「上下要留多高」。
//!
//! 为什么单独抽一层：`DataTable` 已经跑通「固定行高 + 上下 spacer + overscan」，
//! 树与集合是同一个形状（行高固定、按索引切片）。三个组件各写一遍 `floor/ceil`
//! 的边界处理，迟早在 `total=0`、`view_h=0`、`scroll_top` 越界这几个角上分叉。
//!
//! 约定：**行高必须是固定值**。变高列表（聊天流、折叠面板）不在这一层的范围内 ——
//! 那需要前缀和 + 二分查找，属于另一套数据结构。

use std::ops::Range;

/// 视口上下各多渲染几行。太少会在快速滚动时露白，太多就失去虚拟化的意义。
pub const OVERSCAN: usize = 6;

/// 一个渲染窗口：`[start, end)`，下标是**行号**（不是像素）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Window {
    start: usize,
    end: usize,
}

impl Window {
    /// 构造时保证 `start <= end`，别处就不用再防「反了的区间」。
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: end.max(start),
        }
    }

    /// 全量窗口。非虚拟模式与「还没量到视口」时用它。
    pub fn full(total: usize) -> Self {
        Self::new(0, total)
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }

    pub fn slice(&self) -> Range<usize> {
        self.start..self.end
    }

    pub fn contains(&self, row: usize) -> bool {
        row >= self.start && row < self.end
    }

    /// 前面跳过的行占多少像素（上 spacer 的高度）。
    pub fn offset(&self, row_h: f64) -> f64 {
        self.start as f64 * row_h
    }

    /// 后面还剩多少像素（下 spacer 的高度）。
    pub fn tail(&self, total: usize, row_h: f64) -> f64 {
        total.saturating_sub(self.end) as f64 * row_h
    }
}

/// 由滚动位置算窗口。
///
/// `view_h <= 0`（元素还没布局、或 `display:none`）时**退化为全量**：宁可多渲染一帧，
/// 也不要在首帧给出一片空白 —— 空白会让用户以为数据没加载。
pub fn window(scroll_top: f64, view_h: f64, row_h: f64, total: usize, overscan: usize) -> Window {
    if total == 0 {
        return Window::new(0, 0);
    }
    if view_h <= 0.0 || row_h <= 0.0 {
        return Window::full(total);
    }
    let top = scroll_top.max(0.0);
    let first = (top / row_h).floor() as usize;
    let visible = (view_h / row_h).ceil() as usize + 1;
    // `scroll_top` 可能因为内容变短而超出总高：起点也必须夹回 `total`，
    // 否则 `Window::new` 会把 end 抬到 start，得到一片空白。
    let start = first.saturating_sub(overscan).min(total);
    let end = (first + visible + overscan).min(total).max(start);
    Window::new(start, end)
}

/// 直接从滚动容器上量。三个组件都是「元素 + 固定行高」，没必要各自写一遍。
pub fn window_of(el: &web_sys::Element, row_h: f64, total: usize, overscan: usize) -> Window {
    window(
        el.scroll_top() as f64,
        el.client_height() as f64,
        row_h,
        total,
        overscan,
    )
}

/// 数据变少（过滤 / 折叠）后把窗口夹回有效范围，避免 `[s..e]` 越界 panic。
pub fn clamp(win: Window, total: usize) -> Window {
    if total == 0 {
        return Window::new(0, 0);
    }
    let start = win.start.min(total - 1);
    let end = win.end.clamp(start, total);
    Window::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_total_is_empty() {
        let w = window(0.0, 300.0, 28.0, 0, 6);
        assert!(w.is_empty());
        assert_eq!(w.offset(28.0), 0.0);
    }

    #[test]
    fn unmeasured_viewport_renders_all() {
        let w = window(0.0, 0.0, 28.0, 100, 6);
        assert_eq!(w.slice(), 0..100);
    }

    #[test]
    fn window_follows_scroll() {
        // 视口 300 高、行高 28 → ceil(300/28)=11 行，再 +1 兜住半行；overscan 6
        let w = window(0.0, 300.0, 28.0, 1000, 6);
        assert_eq!(w.start(), 0);
        assert_eq!(w.end(), 6 + 12);

        let w = window(280.0, 300.0, 28.0, 1000, 6);
        assert_eq!(w.start(), 10 - 6);
        assert_eq!(w.offset(28.0), 4.0 * 28.0);
    }

    #[test]
    fn window_never_exceeds_total() {
        let w = window(10_000.0, 300.0, 28.0, 50, 6);
        assert_eq!(w.end(), 50);
        assert_eq!(w.tail(50, 28.0), 0.0);
    }

    #[test]
    fn clamp_after_shrink() {
        let w = Window::new(40, 60);
        assert_eq!(clamp(w, 10).slice(), 9..10);
        assert!(clamp(w, 0).is_empty());
    }
}
