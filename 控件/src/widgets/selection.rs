//! 集合类控件的共享选择模型。
//!
//! 为什么单独抽出来：List / DataTable / TreeView / CollectionView 的
//! 「Ctrl 切换、Shift 连选、普通点击替换」是同一套语义。四份复制粘贴之后，
//! 只要有一处把 Shift 的锚点算错，四个控件就会各错各的。
//!
//! 三条约定：
//! 1) 身份是稳定 ID，不是下标 —— 排序、虚拟滚动都不该改动 ID。
//! 2) 焦点与选中分离：方向键只移动焦点，Space / 点击才改变选中。
//! 3) 选中集按「当前可见顺序」输出，调用方拿到的 Vec 顺序是可预期的。

/// 选择模式。`None` 用于只读表格：既不画 `aria-selected`，也不响应点击。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SelectionMode {
    #[default]
    None,
    Single,
    Multiple,
}

impl SelectionMode {
    pub fn is_multiple(self) -> bool {
        matches!(self, SelectionMode::Multiple)
    }

    pub fn is_selectable(self) -> bool {
        !matches!(self, SelectionMode::None)
    }
}

/// 纯数据的选择状态：不持有任何信号，方便在 `RwSignal` 里整体替换。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionState {
    selected: Vec<String>,
    /// Shift 连选的起点。普通点击 / Ctrl 切换都会把锚点移到当前项。
    anchor: Option<String>,
}

impl SelectionState {
    pub fn with_selected(ids: impl IntoIterator<Item = String>) -> Self {
        let selected: Vec<String> = ids.into_iter().collect();
        let anchor = selected.first().cloned();
        Self { selected, anchor }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.selected.iter().any(|k| k == id)
    }

    pub fn selected(&self) -> &[String] {
        &self.selected
    }

    /// 按调用方给的顺序输出（通常就是当前可见行序），集合外的选中项追加在末尾。
    pub fn ordered(&self, order: &[String]) -> Vec<String> {
        let mut out: Vec<String> = order
            .iter()
            .filter(|k| self.selected.iter().any(|s| s == *k))
            .cloned()
            .collect();
        for key in &self.selected {
            if !out.iter().any(|k| k == key) {
                out.push(key.clone());
            }
        }
        out
    }

    /// 单选：替换为 `id`。返回是否真的变了。
    pub fn select_only(&mut self, id: &str) -> bool {
        let changed = self.selected.len() != 1 || self.selected.first().map(String::as_str) != Some(id);
        self.selected = vec![id.to_string()];
        self.anchor = Some(id.to_string());
        changed
    }

    /// Ctrl / Cmd 切换：已选则去掉，未选则追加。锚点跟随到当前项。
    pub fn toggle(&mut self, id: &str) -> bool {
        let changed = if let Some(pos) = self.selected.iter().position(|k| k == id) {
            self.selected.remove(pos);
            true
        } else {
            self.selected.push(id.to_string());
            true
        };
        self.anchor = Some(id.to_string());
        changed
    }

    /// Shift 连选：从锚点到 `id` 的闭区间整体替换。
    fn range_to(&mut self, id: &str, order: &[String]) -> bool {
        let Some(anchor) = self.anchor.clone() else {
            return self.select_only(id);
        };
        let a = order.iter().position(|k| k == &anchor);
        let b = order.iter().position(|k| k == id);
        let (Some(a), Some(b)) = (a, b) else {
            return self.select_only(id);
        };
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let next: Vec<String> = order[lo..=hi].to_vec();
        if self.selected == next {
            return false;
        }
        self.selected = next;
        true
    }

    /// 统一入口：指针 / 键盘的点击意图。
    ///
    /// `toggle` = Ctrl/Cmd，`range` = Shift。单选模式下两个修饰键都退化为「替换」，
    /// 这样调用方不必在外层再判断一次模式。
    pub fn click(
        &mut self,
        mode: SelectionMode,
        id: &str,
        order: &[String],
        toggle: bool,
        range: bool,
    ) -> bool {
        match mode {
            SelectionMode::None => false,
            SelectionMode::Single => self.select_only(id),
            SelectionMode::Multiple => {
                if range {
                    self.range_to(id, order)
                } else if toggle {
                    self.toggle(id)
                } else {
                    self.select_only(id)
                }
            }
        }
    }

    pub fn select_all(&mut self, order: &[String]) -> bool {
        if self.selected == order {
            return false;
        }
        self.selected = order.to_vec();
        if self.anchor.is_none() {
            self.anchor = order.first().cloned();
        }
        true
    }
}
