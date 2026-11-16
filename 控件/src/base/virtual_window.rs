//! 虚拟窗口：只回答两个问题 —— 「该渲染哪几行」「上下要留多高」。
//!
//! 为什么单独抽一层：`DataTable` 已经跑通「固定行高 + 上下 spacer + overscan」，
//! 树与集合是同一个形状（行高固定、按索引切片）。三个组件各写一遍 `floor/ceil`
//! 的边界处理，迟早在 `total=0`、`view_h=0`、`scroll_top` 越界这几个角上分叉。
//!
//! 两套数据结构，一套语义：
//! - 固定行高：`window()` / `window_of()`，一次除法 + 一次乘法，行高是常量；
//! - 变高行（聊天流、折叠面板、多行正文）：`Heights` 存前缀和，`index_at` 二分，
//!   输出 `VarWindow`（行区间 + 上下**像素**留白）。上下留白不能再用「行数 × 行高」，
//!   因为每行高度不同。
//!
//! 两套的退化口径刻意保持一致：`view_h <= 0`、总高为 0 → 全量（宁可多渲染一帧，
//! 不要首帧空白）；`scroll_top` 越界 → 起点夹回有效范围（坑 7：滚到底不能塌成空窗）。

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

/* ------------------------------------------------------------------ 变高行 */

/// 一堆不定高行的高度表：`prefix[i]` = 前 `i` 行之和（`len = n + 1`）。
///
/// 用前缀和而不是「行 → 高度」的数组：窗口只需要两个问题 ——
/// 「第 k 行从哪个像素开始」（`offset`，O(1)）和「这个像素落在第几行」（`index_at`，
/// O(log n) 二分）。逐行累加会把这两件事都变成 O(n)，滚动时每帧都算一遍。
///
/// 高度要夹到「有限且 ≥ 0」：负高度会让前缀和不再单调，二分会给出随机答案。
#[derive(Clone, PartialEq, Debug)]
pub struct Heights {
    prefix: Vec<f64>,
}

impl Heights {
    /// 全部按同一个估计值起手。测量前先用它排一版，量到真高再逐行校正。
    pub fn uniform(n: usize, h: f64) -> Self {
        let h = sane(h);
        Self {
            prefix: (0..=n).map(|i| i as f64 * h).collect(),
        }
    }

    /// 从每行实测高度建表。
    pub fn from_heights(heights: Vec<f64>) -> Self {
        let mut prefix = Vec::with_capacity(heights.len() + 1);
        prefix.push(0.0);
        let mut acc = 0.0;
        for h in heights {
            acc += sane(h);
            prefix.push(acc);
        }
        Self { prefix }
    }

    pub fn len(&self) -> usize {
        self.prefix.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 内容总高（= 最后一行底边）。
    pub fn total(&self) -> f64 {
        self.prefix[self.prefix.len() - 1]
    }

    /// 第 `row` 行的顶边。越界夹到两端，调用方不用先判断。
    pub fn offset(&self, row: usize) -> f64 {
        self.prefix[row.min(self.len())]
    }

    /// 第 `row` 行的高度。越界给 0。
    pub fn height(&self, row: usize) -> f64 {
        if row >= self.len() {
            return 0.0;
        }
        self.prefix[row + 1] - self.prefix[row]
    }

    /// 实测校正一行。
    ///
    /// 前缀和的性质决定了「改一行就要改它后面所有行」：这里把 `row + 1..` 整段
    /// 平移一个差值。O(n) 但只在一行真的量出差异（> 0.5px，由调用方判断）时调用一次，
    /// 而 n 是**总行数**、不是窗口行数 —— 万级数据下每次校正 1 万次浮点加法，
    /// 一帧内几十次也在预算内。要 O(log n) 得换成 Fenwick 树，等真的有 10 万行再说。
    pub fn set(&mut self, row: usize, h: f64) {
        if row >= self.len() {
            return;
        }
        let delta = sane(h) - self.height(row);
        if delta == 0.0 {
            return;
        }
        for p in &mut self.prefix[row + 1..] {
            *p += delta;
        }
    }

    /// 项数变少（过滤 / 折叠）时把表截短，别留上一批数据的脏高度。
    /// 只截断不补齐：变多意味着换了一批数据，调用方应当重建表。
    pub fn truncate(&mut self, n: usize) {
        if n < self.len() {
            self.prefix.truncate(n + 1);
        }
    }

    /// 像素 → 行号：返回**包含** `y` 的那一行，夹到 `[0, len-1]`。
    ///
    /// 负值、`y` 超过总高都夹到两端（坑 7 的另一半：越界不能让窗口塌成空）。
    pub fn index_at(&self, y: f64) -> usize {
        let n = self.len();
        if n == 0 || !(y > 0.0) {
            return 0;
        }
        // prefix[1..] 里有多少个 <= y，答案就是那一行（前缀和单调不减）。
        let k = self.prefix[1..].partition_point(|&p| p <= y);
        k.min(n - 1)
    }

    /// 由滚动位置算变高窗口。
    ///
    /// `overscan_px` 是**像素**不是行数：前几行很高时，按行 overscan 会白渲染一大片。
    /// `end` 多给一行，口径与固定行高版的 `+1 兜住半行` 一致 —— `index_at` 给出的是
    /// 「包含底边的行」，加一是排他上界，再加一是保守行。
    pub fn window(&self, scroll_top: f64, view_h: f64, overscan_px: f64) -> VarWindow {
        let n = self.len();
        if n == 0 {
            return VarWindow::new(0, 0, 0.0, 0.0);
        }
        let total = self.total();
        if view_h <= 0.0 || total <= 0.0 {
            return VarWindow::new(0, n, 0.0, 0.0);
        }
        let top = if scroll_top.is_finite() {
            scroll_top.max(0.0)
        } else {
            0.0
        };
        let ov = if overscan_px.is_finite() {
            overscan_px.max(0.0)
        } else {
            0.0
        };
        let start = self.index_at(top - ov);
        let end = (self.index_at(top + view_h + ov) + 2).min(n).max(start + 1);
        VarWindow::new(start, end, self.offset(start), total - self.offset(end))
    }
}

fn sane(h: f64) -> f64 {
    if h.is_finite() && h > 0.0 {
        h
    } else {
        0.0
    }
}

/// 变高窗口：行区间 `[start, end)` + 上下要留的**像素**（不是行数 × 行高）。
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct VarWindow {
    start: usize,
    end: usize,
    pad_top: f64,
    pad_bottom: f64,
}

impl VarWindow {
    /// 构造时保证 `start <= end`、两个留白非负 —— 别处就不用再防「负的 spacer」。
    pub fn new(start: usize, end: usize, pad_top: f64, pad_bottom: f64) -> Self {
        Self {
            start,
            end: end.max(start),
            pad_top: sane(pad_top),
            pad_bottom: sane(pad_bottom),
        }
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

    /// 上 spacer 的高度 = `Heights::offset(start)`。
    pub fn pad_top(&self) -> f64 {
        self.pad_top
    }

    /// 下 spacer 的高度 = `total - offset(end)`，恒 ≥ 0。
    pub fn pad_bottom(&self) -> f64 {
        self.pad_bottom
    }

    /// 给只认 `Window` 的调用方（键盘导航、`clamp` 之类）。
    pub fn to_window(&self) -> Window {
        Window::new(self.start, self.end)
    }
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

    /* ------------------------------ 变高行 ------------------------------ */

    #[test]
    fn heights_empty_is_empty() {
        let h = Heights::uniform(0, 32.0);
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
        assert_eq!(h.total(), 0.0);
        assert_eq!(h.offset(0), 0.0);
        assert_eq!(h.height(0), 0.0);
        assert_eq!(h.index_at(100.0), 0);
        let w = h.window(0.0, 300.0, 100.0);
        assert!(w.is_empty());
        assert_eq!((w.pad_top(), w.pad_bottom()), (0.0, 0.0));
    }

    #[test]
    fn heights_single_row() {
        let h = Heights::from_heights(vec![50.0]);
        assert_eq!(h.len(), 1);
        assert_eq!(h.total(), 50.0);
        assert_eq!(h.offset(0), 0.0);
        assert_eq!(h.offset(1), 50.0);
        assert_eq!(h.height(0), 50.0);
        // 越界行一律给 0，不 panic
        assert_eq!(h.offset(9), 50.0);
        assert_eq!(h.height(9), 0.0);
        let w = h.window(0.0, 300.0, 100.0);
        assert_eq!(w.slice(), 0..1);
        assert_eq!((w.pad_top(), w.pad_bottom()), (0.0, 0.0));
    }

    #[test]
    fn heights_all_zero_falls_back_to_full() {
        // 全零高度 = 还没量到任何东西：总高 0，退化为全量，别给空窗
        let h = Heights::uniform(5, 0.0);
        assert_eq!(h.total(), 0.0);
        let w = h.window(0.0, 300.0, 120.0);
        assert_eq!(w.slice(), 0..5);
        assert_eq!((w.pad_top(), w.pad_bottom()), (0.0, 0.0));
    }

    #[test]
    fn var_window_unmeasured_viewport_renders_all() {
        let h = Heights::uniform(100, 28.0);
        assert_eq!(h.window(0.0, 0.0, 168.0).slice(), 0..100);
        assert_eq!(h.window(500.0, -1.0, 168.0).slice(), 0..100);
    }

    #[test]
    fn var_window_matches_fixed_for_uniform_heights() {
        // 行对齐的滚动位置：像素 overscan = 行数 × 行高，两套必须给出同一个 slice
        // 和同一组留白（`VarWindow` 的 pad 就是旧 `Window::offset/tail`）。
        let h = Heights::uniform(1000, 28.0);
        for top in [0.0, 280.0, 560.0, 9800.0] {
            let var = h.window(top, 300.0, 6.0 * 28.0);
            let fixed = window(top, 300.0, 28.0, 1000, 6);
            assert_eq!(var.slice(), fixed.slice(), "top = {top}");
            assert_eq!(var.pad_top(), fixed.offset(28.0), "top = {top}");
            assert_eq!(var.pad_bottom(), fixed.tail(1000, 28.0), "top = {top}");
        }
    }

    #[test]
    fn var_window_clamps_when_scroll_is_past_the_end() {
        // 内容变短、scrollTop 还没被浏览器夹回来：不许塌成空窗（坑 7）
        let h = Heights::from_heights(vec![10.0; 50]);
        let w = h.window(10_000.0, 300.0, 60.0);
        assert!(!w.is_empty());
        assert_eq!(w.slice(), 49..50);
        assert_eq!(w.pad_bottom(), 0.0);
        assert_eq!(w.pad_top(), 490.0);
    }

    #[test]
    fn index_at_hits_boundaries_and_inside() {
        // prefix = [0, 10, 30, 60]
        let h = Heights::from_heights(vec![10.0, 20.0, 30.0]);
        assert_eq!(h.index_at(-1.0), 0);
        assert_eq!(h.index_at(0.0), 0);
        assert_eq!(h.index_at(9.9), 0);
        assert_eq!(h.index_at(10.0), 1); // 正好落在行边界 → 下一行
        assert_eq!(h.index_at(29.9), 1);
        assert_eq!(h.index_at(30.0), 2);
        assert_eq!(h.index_at(59.9), 2);
        assert_eq!(h.index_at(60.0), 2); // 等于 total 也夹在最后一行
        assert_eq!(h.index_at(999.0), 2);
    }

    #[test]
    fn set_keeps_total_and_offsets_consistent() {
        let mut h = Heights::from_heights(vec![10.0, 20.0, 30.0]);
        h.set(1, 25.0);
        assert_eq!(h.total(), 65.0);
        assert_eq!(h.offset(0), 0.0);
        assert_eq!(h.offset(1), 10.0);
        assert_eq!(h.offset(2), 35.0);
        assert_eq!(h.offset(3), 65.0);
        assert_eq!(h.height(1), 25.0);
        // 不改别的行
        assert_eq!((h.height(0), h.height(2)), (10.0, 30.0));
        // 负值 / NaN 夹到 0（不是「保持原值」，也不是让前缀和变成非单调），
        // 越界行直接忽略。
        h.set(0, -5.0);
        h.set(2, f64::NAN);
        h.set(9, 40.0);
        assert_eq!(h.total(), 25.0);
        assert_eq!((h.height(0), h.height(2)), (0.0, 0.0));
        assert_eq!(h.offset(1), 0.0);
        assert!(h.offset(3).is_finite());
    }

    #[test]
    fn pads_always_fill_the_gap_to_total() {
        // pad_top + 窗口内容 + pad_bottom == total，任何滚动位置都成立；
        // 这也是「上下撑高行」能拼回完整滚动高度的前提。
        let h = Heights::from_heights((0..200).map(|i| 18.0 + (i % 7) as f64 * 9.0).collect());
        for top in [0.0, 37.0, 500.0, 2_000.0, 99_999.0] {
            let w = h.window(top, 420.0, 120.0);
            let content = h.offset(w.end()) - h.offset(w.start());
            let sum = w.pad_top() + content + w.pad_bottom();
            assert!((sum - h.total()).abs() < 1e-6, "top = {top}: {sum} != {}", h.total());
            assert!(w.pad_top() >= 0.0 && w.pad_bottom() >= 0.0);
        }
    }

    #[test]
    fn overscan_is_pixels_not_rows() {
        // 前 3 行各 300px：按 6 **行** overscan 会把 top=900 的窗口拉到第 0 行，
        // 白渲染 900px 内容；按 20 **像素**只多渲染一行。
        let mut hs = vec![300.0, 300.0, 300.0];
        hs.extend(std::iter::repeat(20.0).take(100));
        let h = Heights::from_heights(hs);
        let w = h.window(900.0, 100.0, 20.0);
        assert_eq!(w.start(), 2);
        assert_eq!(w.pad_top(), 600.0);
        // 对照：同样的 overscan 当成 6 行（行高取第一行的 300）→ 起点被拉回 0，
        // 多出来的三行 900px 全是白渲染。
        assert_eq!(window(900.0, 100.0, 300.0, 103, 6).start(), 0);
    }

    #[test]
    fn truncate_drops_stale_tail() {
        let mut h = Heights::uniform(10, 30.0);
        h.set(0, 80.0);
        assert_eq!(h.total(), 80.0 + 9.0 * 30.0);
        h.truncate(3);
        assert_eq!(h.len(), 3);
        assert_eq!(h.total(), 80.0 + 2.0 * 30.0);
        // 截断后再算窗口不会越界
        let w = h.window(0.0, 300.0, 90.0);
        assert_eq!(w.slice(), 0..3);
    }
}
