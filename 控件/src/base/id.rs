use std::sync::atomic::{AtomicUsize, Ordering};

static SEQ: AtomicUsize = AtomicUsize::new(0);

/// 生成稳定到一次渲染的 DOM id（用于 aria-controls / aria-labelledby）。
pub fn next_id(prefix: &str) -> String {
    format!("{prefix}-{}", SEQ.fetch_add(1, Ordering::Relaxed) + 1)
}
