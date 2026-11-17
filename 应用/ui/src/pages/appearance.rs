use glass_design::prelude::*;
use leptos::prelude::*;

/// 外观分区：颜色井、取色器、文件选择、拖放。
#[component]
pub fn AppearancePage() -> impl IntoView {
    // —— 颜色 ——
    let well = RwSignal::new("#0a84ff".to_string());
    let preset = RwSignal::new("#ff9f0a".to_string());
    let inline_picker = RwSignal::new("#30d158".to_string());
    // —— 独立取色器 ——
    let full = RwSignal::new("#bf5af2".to_string());
    // —— 文件 ——
    let attach_note = RwSignal::new(String::new());
    let single_note = RwSignal::new(String::new());
    // —— 拖放 ——
    let dropped = RwSignal::new(0usize);

    let presets = SwatchItem::accents();

    view! {
        <div class="gd-page">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"外观"</h1>
                    <p class="c-subtitle" style="white-space:normal">
                        "颜色井、取色器、文件选择与拖放：值都在调用方手里，控件只上报。"
                    </p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"颜色"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--6">
                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"颜色井：点开是浮层取色器，Esc 或点外关闭"</span>
                                <div class="l-row l-row--3" style="align-items:center">
                                    <ColorWell
                                        value=Signal::derive(move || well.get())
                                        label="强调色"
                                        show_label=true
                                        show_hex=true
                                        presets=presets.clone()
                                        on_change=Callback::new(move |hex: String| well.set(hex))
                                    />
                                    <span class="c-subtitle">{move || format!("当前 {}", well.get())}</span>
                                </div>
                            </div>

                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"预设：单色块可选中，选中环由 aria-checked 驱动"</span>
                                <Swatches
                                    items=presets.clone()
                                    selected=Signal::derive(move || preset.get())
                                    label="预设颜色"
                                    on_select=Callback::new(move |hex: String| preset.set(hex))
                                />
                                <span class="c-subtitle">{move || format!("选中 {}", preset.get())}</span>
                            </div>

                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"内嵌取色器 · 固定宽 240"</span>
                                <ColorPicker
                                    value=Signal::derive(move || inline_picker.get())
                                    width=240
                                    presets=presets.clone()
                                    label="内嵌取色器"
                                    on_change=Callback::new(move |hex: String| inline_picker.set(hex))
                                />
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"颜色选择器"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--6" style="align-items:flex-start;flex-wrap:wrap">
                            <div style="width:236px">
                                <ColorPicker
                                    value=Signal::derive(move || full.get())
                                    label="颜色"
                                    on_change=Callback::new(move |hex: String| full.set(hex))
                                />
                            </div>
                            <div class="l-stack l-stack--2" style="min-width:200px;max-width:320px">
                                <span class="c-subtitle">"实时读数 · 每次拖拽都上报"</span>
                                <span class="c-colorpicker__hextext selectable">
                                    {move || full.get()}
                                </span>
                                <span class="c-subtitle">
                                    {move || match hex_to_rgb(&full.get()) {
                                        Some((r, g, b)) => format!("rgb({r}, {g}, {b})"),
                                        None => "rgb(—)".to_string(),
                                    }}
                                </span>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"文件"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--6">
                            <div class="l-stack l-stack--2" style="width:420px;max-width:100%">
                                <span class="c-subtitle">"多选 · 最多 4 个 · 单个 5 MB · 只收图片与 PDF"</span>
                                <FilePicker
                                    label="上传附件"
                                    accept="image/*,.pdf"
                                    multiple=true
                                    max_files=4
                                    max_size=5 * 1024 * 1024
                                    on_change=Callback::new(move |files: Vec<FileItem>| {
                                        attach_note.set(format!("最近一次选择 {} 个文件", files.len()));
                                    })
                                />
                                <span class="c-subtitle">{move || attach_note.get()}</span>
                            </div>

                            <div class="l-stack l-stack--2" style="width:280px;max-width:100%">
                                <span class="c-subtitle">"单选 · 只收 PDF"</span>
                                <FilePicker
                                    label="选择文档"
                                    accept=".pdf"
                                    on_change=Callback::new(move |files: Vec<FileItem>| {
                                        single_note.set(match files.first() {
                                            Some(f) => format!("{} · {}", f.name, f.size_text()),
                                            None => "已清空".to_string(),
                                        });
                                    })
                                />
                                <span class="c-subtitle">{move || single_note.get()}</span>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"拖放"</SectionTitle>
                    <Card>
                        <div class="l-pad l-row l-row--6" style="align-items:flex-start;flex-wrap:wrap">
                            <div style="flex:1;min-width:300px;max-width:420px">
                                <DropZone
                                    label="把图片拖到这里"
                                    hint="松开即可添加"
                                    accept="image/*"
                                    max_size=10 * 1024 * 1024
                                    on_change=Callback::new(move |files: Vec<FileItem>| {
                                        dropped.update(|n| *n += files.len());
                                    })
                                />
                            </div>
                            <div class="l-stack l-stack--2">
                                <span class="c-subtitle">"已选数量"</span>
                                <span class="c-colorpicker__hextext">
                                    {move || dropped.get().to_string()}
                                </span>
                            </div>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}
