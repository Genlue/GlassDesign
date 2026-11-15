use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::base::icon::{Icon, IconName};
use crate::base::id::next_id;

/* ==========================================================================
   文件选择 FilePicker / DropZone

   两个控件只解决「选到文件」，不解决「传」：上传是任务，要进度、要重试、
   要能取消，那是另一套东西。这里把错误文案留在控件下方，任务状态交给调用方。

   它们只持有 `FileList` 派生的元数据（名字/大小/类型）+ 原始 `File` 句柄，
   不读内容、不上传 —— 一旦开始读内容，就会牵出编码、内存与「读了一半被移除」
   这些问题，而它们都不属于一个选择器。
   ========================================================================== */

/// 一个已选文件。`file` 是原始句柄，调用方拿它去构造 `FormData` 或交给
/// `FileReader`；控件自己永远不碰它的内容。
#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    /// 字节
    pub size: f64,
    /// MIME（浏览器给不出时是空串）
    pub mime: String,
    pub file: web_sys::File,
}

impl FileItem {
    pub fn size_text(&self) -> String {
        format_size(self.size)
    }
}

/// 文件大小：`B / KB / MB / GB`，KB 以上保留一位小数。
///
/// 为什么不用 1000 进制：文件管理器、`du`、系统选择器都用 1024，混用会让
/// 「明明是 4.8 MB 却被说超过 5 MB」这种争执出现。
fn format_size(bytes: f64) -> String {
    let b = bytes.max(0.0);
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b < KB {
        format!("{b:.0} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.1} GB", b / GB)
    }
}

/// 打开系统选择器。
///
/// `HtmlInputElement::click()` 必须在用户手势的同步调用栈里，所以这里只做
/// 一件事：点那个隐藏 input。`change` 监听在组件挂载时已经绑好，不在点击时
/// 临时挂 —— 临时挂会漏掉「对话框开着时组件被重建」的那一次。
fn pick_files(input: &NodeRef<html::Input>) {
    if let Some(el) = input.get() {
        let _ = el.click();
    }
}

/// 把 `FileList` 读成快照。`FileList` 是「活的」对象，对话框再开一次就可能
/// 变空，所以先取出 `File` 句柄再返回。
fn read_files(list: Option<web_sys::FileList>) -> Vec<web_sys::File> {
    let Some(list) = list else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i))
        .collect::<Vec<_>>()
}

/// `accept` 的三类写法：扩展名 `.png`、通配 `image/*`、精确 `image/png`。
fn matches_accept(name: &str, mime: &str, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    let name = name.to_lowercase();
    let mime = mime.to_lowercase();
    tokens.iter().any(|t| {
        if let Some(ext) = t.strip_prefix('.') {
            name.ends_with(&format!(".{ext}"))
        } else if let Some(prefix) = t.strip_suffix("/*") {
            mime.starts_with(&format!("{prefix}/"))
        } else if t == "*" || t == "*/*" {
            true
        } else {
            mime == *t
        }
    })
}

/// 逐文件判定：合格留下，不合格带原因退回。
///
/// 为什么是逐文件而不是整批：一批里混进一个超限文件，不该把另外两个合格的
/// 一起扔掉 —— 用户还得重新选一遍那两个。
fn accept_files(
    files: Vec<web_sys::File>,
    accept: &str,
    limit: usize,
    max_size: u64,
) -> (Vec<FileItem>, Vec<String>) {
    let tokens: Vec<String> = accept
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    let mut ok = Vec::new();
    let mut errs = Vec::new();
    for f in files {
        let name = f.name();
        if !matches_accept(&name, &f.type_(), &tokens) {
            errs.push(format!("{name}：类型不支持"));
            continue;
        }
        if max_size > 0 && f.size() > max_size as f64 {
            errs.push(format!("{name}：超过 {}", format_size(max_size as f64)));
            continue;
        }
        if limit > 0 && ok.len() >= limit {
            errs.push(format!("{name}：最多 {limit} 个"));
            continue;
        }
        ok.push(FileItem {
            name,
            size: f.size(),
            mime: f.type_(),
            file: f,
        });
    }
    (ok, errs)
}

/// 拖放里有没有文件。`dataTransfer.types` 是一个字符串数组，只有含 `Files`
/// 才是文件拖拽（拖文字、拖链接不该点亮放置区）。
fn has_files(ev: &web_sys::DragEvent) -> bool {
    ev.data_transfer()
        .map(|dt| {
            dt.types()
                .includes(&JsValue::from_str("Files"), 0)
        })
        .unwrap_or(false)
}

/* ==========================================================================
   文件选择
   ========================================================================== */

/// 文件选择：一颗按钮打开系统对话框，下方是已选清单。
///
/// 一次选择**替换**整批，而不是追加 —— 这和系统对话框的语义一致（它就是
/// 「这一批」），追加会让「取消勾选某个文件」变成不可能。
#[component]
pub fn FilePicker(
    #[prop(into, optional)] label: String,
    /// 形如 `image/*,.pdf`；空表示不限
    #[prop(into, optional)] accept: String,
    #[prop(optional)] multiple: bool,
    /// 0 = 不限
    #[prop(optional)] max_files: usize,
    /// 单文件字节上限，0 = 不限
    #[prop(optional)] max_size: u64,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<Vec<FileItem>>>,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();
    let btn_ref = NodeRef::<html::Button>::new();
    let files = RwSignal::new(Vec::<FileItem>::new());
    let error = RwSignal::new(String::new());
    let error_id = next_id("gd-filepicker-error");
    let error_id_attr = error_id.clone();
    // 单选天然只能一个：把 multiple 折进数字上限，校验就只剩一套判断
    let limit = if multiple { max_files } else { 1 };
    let accept_for_change = accept.clone();
    let base = if label.is_empty() {
        "选择文件".to_string()
    } else {
        label.clone()
    };

    let text = move || {
        let list = files.get();
        match list.len() {
            0 => base.clone(),
            1 => list[0].name.clone(),
            n => format!("已选 {n} 个文件"),
        }
    };

    let remove = move |index: usize| {
        files.update(|list| {
            if index < list.len() {
                list.remove(index);
            }
        });
        error.set(String::new());
        if let Some(cb) = on_change {
            cb.run(files.get_untracked());
        }
        // 被点的移除按钮已经不在文档里，焦点会掉到 body；交还浏览按钮
        if let Some(el) = btn_ref.get() {
            let _ = el.focus();
        }
    };

    view! {
        <div class="l-stack l-stack--2">
            <button
                class="c-btn c-filepicker"
                type="button"
                aria-invalid=move || (!error.get().is_empty()).to_string()
                aria-describedby=move || (!error.get().is_empty()).then(|| error_id_attr.clone())
                title=move || {
                    // 按钮文字截断在 24ch，完整名单走 title
                    let list = files.get();
                    match list.len() {
                        0 => String::new(),
                        1 => list[0].name.clone(),
                        _ => list.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join("\n"),
                    }
                }
                disabled=disabled
                node_ref=btn_ref
                on:click=move |_| pick_files(&input_ref)
            >
                <span class="c-filepicker__glyph" aria-hidden="true">
                    <Icon name=IconName::UPLOAD size=14 />
                </span>
                <span class="c-filepicker__label">{text}</span>
            </button>

            <input
                type="file"
                hidden
                tabindex="-1"
                aria-hidden="true"
                node_ref=input_ref
                multiple=multiple
                accept=(!accept.is_empty()).then(|| accept.clone())
                on:change=move |_| {
                    let Some(el) = input_ref.get() else { return };
                    let picked = read_files(el.files());
                    // 必须清空 value：不清的话同一个文件再选一次不会再触发 change
                    el.set_value("");
                    if picked.is_empty() {
                        return;
                    }
                    let (items, errs) =
                        accept_files(picked, &accept_for_change, limit, max_size);
                    // 合格的要留下来，不合格的写进错误行 —— 见 accept_files 的注释
                    error.set(errs.join("；"));
                    if items.is_empty() {
                        return;
                    }
                    files.set(items.clone());
                    if let Some(cb) = on_change {
                        cb.run(items);
                    }
                }
            />

            {move || {
                let list = files.get();
                (!list.is_empty()).then(|| view! {
                    <ul class="c-filepicker__list" aria-label="已选文件">
                        {list
                            .iter()
                            .enumerate()
                            .map(|(i, item)| {
                                let name = item.name.clone();
                                view! {
                                    <li class="c-filepicker__item">
                                        <span class="c-filepicker__item-name" title=name.clone()>
                                            {name.clone()}
                                        </span>
                                        <span class="c-filepicker__item-size">
                                            {format_size(item.size)}
                                        </span>
                                        <button
                                            class="c-filepicker__remove"
                                            type="button"
                                            aria-label=format!("移除 {name}")
                                            title=format!("移除 {name}")
                                            on:click=move |_| remove(i)
                                        >
                                            <span class="c-filepicker__remove-icon" aria-hidden="true">
                                                <Icon name=IconName::X size=11 />
                                            </span>
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                })
            }}

            {move || {
                let msg = error.get();
                (!msg.is_empty()).then(|| view! {
                    <div class="c-filepicker__error" role="alert" id=error_id.clone()>{msg}</div>
                })
            }}
        </div>
    }
}

/* ==========================================================================
   拖放
   ========================================================================== */

/// 拖放区：拖入高亮、松开取文件、点击等同选择。
///
/// 高亮靠**进入计数**而不是 `dragenter/dragleave` 这一对：指针划过区域里的
/// 文字或按钮都会各发一次 enter/leave，只看单次事件高亮会一路闪。
///
/// 区域本身是 `div`（里面要放一颗真按钮），所以不给它加 `tabindex` 冒充
/// 按钮 —— 键盘用户的入口是 `__browse`。
#[component]
pub fn DropZone(
    /// 主文案
    #[prop(into, optional)] label: String,
    #[prop(into, optional)] hint: String,
    /// 浏览按钮的文案
    #[prop(into, optional)] browse: String,
    #[prop(into, optional)] accept: String,
    #[prop(default = true)] multiple: bool,
    #[prop(optional)] max_files: usize,
    #[prop(optional)] max_size: u64,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] on_change: Option<Callback<Vec<FileItem>>>,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();
    let over = RwSignal::new(false);
    let depth = RwSignal::new(0i32);
    let error = RwSignal::new(String::new());
    let error_id = next_id("gd-dropzone-error");
    let error_id_attr = error_id.clone();
    let limit = if multiple { max_files } else { 1 };
    // 捕获成 Copy 句柄：`take` 会被 drop 与 change 两处闭包同时持有，
    // 直接捕获 String 会被第一次 move 吃掉。
    let accept = StoredValue::new(accept.clone());

    let main = if label.is_empty() {
        "把文件拖到这里".to_string()
    } else {
        label.clone()
    };
    let sub = if hint.is_empty() {
        "或点击选择文件".to_string()
    } else {
        hint.clone()
    };
    let browse_text = if browse.is_empty() {
        "浏览…".to_string()
    } else {
        browse.clone()
    };

    let take = move |picked: Vec<web_sys::File>| {
        if picked.is_empty() {
            return;
        }
        let (items, errs) = accept_files(picked, &accept.get_value(), limit, max_size);
        error.set(errs.join("；"));
        if !items.is_empty() {
            if let Some(cb) = on_change {
                cb.run(items);
            }
        }
    };

    let reset = move || {
        depth.set(0);
        over.set(false);
    };

    view! {
        // 错误文案是区域的兄弟节点：塞进虚线框会被当成「拖到这里」的一部分念出来
        <div class="l-stack l-stack--2">
        <div
            class="c-dropzone"
            role="group"
            aria-label=main.clone()
            aria-invalid=move || (!error.get().is_empty()).to_string()
            aria-describedby=move || (!error.get().is_empty()).then(|| error_id_attr.clone())
            aria-disabled=disabled.then_some("true")
            // data-state 是既有 CSS 的钩子（idle/over/disabled），data-over 是
            // 对外契约用的布尔别名，两者同写：样式不用改，测试只认一个属性。
            data-state=move || {
                if disabled {
                    "disabled"
                } else if over.get() {
                    "over"
                } else {
                    "idle"
                }
            }
            data-over=move || over.get().then_some("1")
            on:click=move |_| {
                if !disabled {
                    pick_files(&input_ref);
                }
            }
            on:dragenter=move |ev: web_sys::DragEvent| {
                if disabled || !has_files(&ev) {
                    return;
                }
                ev.prevent_default();
                depth.update(|d| *d += 1);
                over.set(true);
            }
            on:dragover=move |ev: web_sys::DragEvent| {
                if disabled {
                    return;
                }
                // 不 preventDefault 浏览器既不认这里是放置目标，也不会派发 drop
                ev.prevent_default();
                if let Some(dt) = ev.data_transfer() {
                    dt.set_drop_effect("copy");
                }
                // 有些浏览器先给 dragover 后给 dragenter，这里补一次计数
                if depth.get_untracked() == 0 {
                    depth.set(1);
                }
                over.set(true);
            }
            on:dragleave=move |ev: web_sys::DragEvent| {
                if disabled {
                    return;
                }
                depth.update(|d| *d -= 1);
                // related_target 为空 = 真的离开了窗口；这时计数不可靠，直接复位
                if depth.get_untracked() <= 0 || ev.related_target().is_none() {
                    reset();
                }
            }
            on:drop=move |ev: web_sys::DragEvent| {
                if disabled {
                    return;
                }
                // 不拦的话浏览器会把文件当成导航目标直接打开
                ev.prevent_default();
                let picked = ev
                    .data_transfer()
                    .map(|dt| read_files(dt.files()))
                    .unwrap_or_default();
                reset();
                take(picked);
            }
        >
            <div class="c-dropzone__label">{main.clone()}</div>
            <div class="c-dropzone__hint">{sub.clone()}</div>
            <button
                class="c-btn c-btn--sm c-dropzone__browse"
                type="button"
                disabled=disabled
            >
                {browse_text.clone()}
            </button>

            <input
                type="file"
                hidden
                tabindex="-1"
                aria-hidden="true"
                node_ref=input_ref
                multiple=multiple
                accept=(!accept.get_value().is_empty()).then(|| accept.get_value())
                on:change=move |_| {
                    let Some(el) = input_ref.get() else { return };
                    let picked = read_files(el.files());
                    el.set_value("");
                    take(picked);
                }
            />
        </div>

        {move || {
            let msg = error.get();
            (!msg.is_empty()).then(|| view! {
                <div class="c-dropzone__error" role="alert" id=error_id.clone()>{msg}</div>
            })
        }}
        </div>
    }
}
