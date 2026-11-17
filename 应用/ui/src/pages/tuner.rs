//! 外观参数调节页 —— 按玻璃面分别调。
//!
//! 三条边界决定了这个页面能做什么：
//!
//! 1. **只改当前页面的 CSS 变量。** 不写盘、不进源码。调完把输出框里的那几行
//!    抄进 `控件/css/src/optics.css`，才算真的改默认值。
//! 2. **参数分两类。** `--glass-w` / `--glass-blur-k` / `--lg-coating` /
//!    `--lg-dispersion` / `--lg-stroke-alpha` / `--lg-vibrancy` / `--hl-bottom`
//!    是 CSS 真读的，拖了立刻变；而 `--lg-band` / `--lg-amount` / `--lg-depth` /
//!    `--lg-stroke-half` / `--lg-stroke-feather` / `--lg-falloff` / `--lg-light` /
//!    `--lg-sheen` / `--lg-sheen-depth` 会被 `build-glass.mjs` 烘进位移与高光贴图，
//!    运行期改它们没有任何效果 —— 所以只读展示，不放假滑块。
//! 3. **初值从 DOM 实测。** 每块面的参数不是写死在这里，而是现场建一个探针元素、
//!    读 `getComputedStyle`。所以打开页面不会有任何视觉变化，只有拖过的那些参数
//!    会进输出框。

use glass_design::prelude::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 八块玻璃面。顺序 = 段控顺序 = 参数矩阵的行。
struct Surf {
    label: &'static str,
    /// 探针与覆盖用的选择器（也是样张要挂的类）
    sel: &'static str,
    note: &'static str,
}

const SURFACES: [Surf; 8] = [
    Surf { label: "顶栏", sel: ".c-glass-chrome", note: "L1 · 浮在内容之上，最轻的一档" },
    Surf { label: "面板", sel: ".c-glass-panel", note: "L2 · 压在内容上的面板" },
    Surf { label: "胶囊", sel: ".c-capsule", note: "L3 · 顶栏里的玻璃胶囊" },
    Surf { label: "停靠栏", sel: ".c-dock__pill", note: "L3 · 悬浮导航的药丸" },
    Surf { label: "菜单", sel: ".c-menu", note: "L2 底盘 · 外影按弹层那一档" },
    Surf { label: "提示条", sel: ".c-toast", note: "L3 · 全局提示" },
    Surf { label: "模态", sel: ".c-glass-sheet", note: "L4 · 最厚的一档，自带外影" },
    Surf { label: "拇指", sel: ".c-slider__thumb", note: "第八块玻璃 · 面积最小" },
];

/// 可实时调的参数。`var` 必须真的被 CSS 读，否则就是空操作。
struct Knob {
    var: &'static str,
    label: &'static str,
    hint: &'static str,
    min: f64,
    max: f64,
    step: f64,
}

const KNOBS: [Knob; 8] = [
    Knob { var: "--glass-w", label: "权重", hint: "高光与色散的整体倍率", min: 0.0, max: 2.0, step: 0.05 },
    Knob { var: "--glass-blur-k", label: "模糊倍率", hint: "乘在该面基础模糊上，1 = 原值", min: 0.0, max: 3.0, step: 0.05 },
    Knob { var: "--lg-coating", label: "涂层", hint: "玻璃本体的染色厚度，越大越不透", min: 0.0, max: 1.0, step: 0.01 },
    Knob { var: "--lg-dispersion", label: "色散", hint: "左右边缘暖进冷出的强度，0 = 关", min: 0.0, max: 1.0, step: 0.01 },
    Knob { var: "--lg-stroke-alpha", label: "描边", hint: "0.8px 镜面环与内壁白边的亮度", min: 0.0, max: 1.0, step: 0.01 },
    Knob { var: "--lg-bloom", label: "顶部晕光", hint: "上沿那道往下晕开的白光，与描边分开", min: 0.0, max: 1.0, step: 0.01 },
    Knob { var: "--lg-vibrancy", label: "饱和", hint: "backdrop 的 saturate 倍率", min: 1.0, max: 2.6, step: 0.05 },
    Knob { var: "--hl-bottom", label: "底部高光", hint: "底部聚光相对顶部的比例", min: 0.0, max: 1.0, step: 0.01 },
];

/// 烘进贴图的参数：只读展示，改了要重跑 build-glass。
const BAKED: [(&str, &str); 9] = [
    ("--lg-band", "折射带宽"),
    ("--lg-amount", "折射位移"),
    ("--lg-depth", "透镜深度"),
    ("--lg-stroke-half", "描边半宽"),
    ("--lg-stroke-feather", "描边羽化"),
    ("--lg-falloff", "边缘衰减"),
    ("--lg-light", "光源角"),
    ("--lg-sheen", "镜面光泽"),
    ("--lg-sheen-depth", "光泽深度"),
];

const NS: usize = SURFACES.len();
const NP: usize = KNOBS.len();

#[component]
pub fn TunerPage() -> impl IntoView {
    let theme = use_context::<Theme>().unwrap_or_else(Theme::provide);

    // 实测初值；NaN = 这块面没有这个参数（例如胶囊没有 --glass-w）。
    let (vals0, base, baked) = init_matrix();
    let na: Vec<bool> = vals0.iter().map(|v| v.is_nan()).collect();
    let base_for_css = base.clone();
    let base_for_reset = base.clone();
    let vals = RwSignal::new(vals0);
    let active = RwSignal::new(0usize);

    // 全局量
    let mode = RwSignal::new(match theme.mode.get_untracked() {
        ThemeMode::Light => "light".to_string(),
        ThemeMode::Dark => "dark".to_string(),
        ThemeMode::System => "system".to_string(),
    });
    let shadow = RwSignal::new(theme.shadow.get_untracked() as f64);
    let stroke = RwSignal::new(theme.stroke.get_untracked() as f64);
    let veil = RwSignal::new(64.0_f64);

    Effect::new(move |_| {
        theme.set_mode(match mode.get().as_str() {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
        });
    });
    Effect::new(move |_| theme.shadow.set(shadow.get() as f32));
    Effect::new(move |_| theme.stroke.set(stroke.get() as f32));
    Effect::new(move |_| set_rule(".l-main", &format!("--veil-h:{}px", veil.get())));

    // 只输出「和初值不一样」的项：输出框就是能直接抄回 optics.css 的那几行。
    // 输出框默认只列「改过的项」—— 那是拿去覆盖默认值用的最小差集。
    // 但第一次看的人会以为漏了参数，所以给一个「导出全部」的开关。
    let full = RwSignal::new(false);
    let css_text = Signal::derive(move || build_css(&vals.get(), &base_for_css, full.get()));
    Effect::new(move |_| apply_tuner_style(&css_text.get()));

    let copy_all = Callback::new(move |_| {
        copy_text(&css_text.get_untracked());
        toast("参数已复制");
    });

    // 把当前面与初值的差，复制到其它面（只写那些面确实有的列）。
    let spread = Callback::new(move |_| {
        let s = active.get_untracked();
        let cur = vals.get_untracked();
        let mut next = cur.clone();
        for p in 0..NP {
            let v = cur[s * NP + p];
            if v.is_nan() {
                continue;
            }
            for t in 0..NS {
                if !next[t * NP + p].is_nan() {
                    next[t * NP + p] = v;
                }
            }
        }
        vals.set(next);
        toast("已复制到全部玻璃面");
    });

    let surface_options: Vec<SegmentedOption> = SURFACES
        .iter()
        .map(|s| SegmentedOption::new(s.sel, s.label))
        .collect();

    let reset = Callback::new(move |_| {
        vals.set(base_for_reset.clone());
        toast("已恢复默认");
    });

    // 「哪些格是空的」是静态事实（`na` 在顶部算好），绝不能从 `vals` 里现读 ——
    // 一旦这层订阅了 `vals`，拖一下滑块就会重建整列 Slider，
    // 拖动状态跟着元素一起没了，表现为「拖一格就断」。
    let rows = move || {
        let s = active.get();
        KNOBS
            .iter()
            .enumerate()
            .filter(|(p, _)| !na[s * NP + p])
            .map(|(p, k)| {
                view! {
                    <Row label=k.label hint=k.hint>
                        <div class="gd-knob">
                            <div class="gd-knob__track">
                                <Slider
                                    value=Signal::derive(move || vals.get()[s * NP + p])
                                    min=k.min
                                    max=k.max
                                    step=k.step
                                    label=k.var
                                    on_input=Callback::new(move |v: f64| {
                                        vals.update(|m| m[s * NP + p] = v);
                                    })
                                />
                            </div>
                            <span class="gd-knob__value">
                                {move || format!("{:.2}", vals.get()[s * NP + p])}
                            </span>
                            <span class="gd-knob__key">{k.var}</span>
                        </div>
                    </Row>
                }
            })
            .collect_view()
    };

    let baked_rows = move || {
        let s = active.get();
        BAKED
            .iter()
            .map(|(var, label)| {
                let v = baked[s]
                    .iter()
                    .find(|(k, _)| k == var)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                view! {
                    <div class="gd-baked">
                        <span class="gd-baked__label">{*label}</span>
                        <span class="gd-baked__val">{v}</span>
                        <span class="gd-baked__key">{*var}</span>
                    </div>
                }
            })
            .collect_view()
    };

    view! {
        <div class="gd-page">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"外观参数"</h1>
                    <p class="c-subtitle" style="white-space:normal">
                        "八块玻璃各自一套参数。拖完把「输出」里那几行发出来，"
                        "由维护者写进 控件/css/src/optics.css 成为默认值。"
                    </p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"全局"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Row label="外观" hint="只改 html 根元素的属性，不动样式规则">
                                <div class="gd-knob">
                                    <Segmented
                                        options=vec![
                                            SegmentedOption::new("light", "浅色"),
                                            SegmentedOption::new("dark", "深色"),
                                            SegmentedOption::new("system", "跟随系统"),
                                        ]
                                        value=Signal::derive(move || mode.get())
                                        on_change=Callback::new(move |v: String| mode.set(v))
                                    />
                                </div>
                            </Row>
                            <Row label="阴影浓度" hint="窗口 / 卡片 / 浮层 / 胶囊共用一个系数">
                                <KnobRow sig=shadow min=0.0 max=2.0 step=0.05 var="--shadow-k" />
                            </Row>
                            <Row label="图标线宽" hint="24 视框下的 stroke-width">
                                <KnobRow sig=stroke min=1.0 max=2.4 step=0.05 var="--ic-stroke" />
                            </Row>
                            <Row label="顶部面纱高度" hint="渐进模糊那一条，0 = 关掉">
                                <KnobRow sig=veil min=0.0 max=120.0 step=2.0 var="--veil-h" unit="px" />
                            </Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"玻璃面"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Segmented
                                options=surface_options
                                value=Signal::derive(move || SURFACES[active.get()].sel.to_string())
                                on_change=Callback::new(move |v: String| {
                                    if let Some(i) = SURFACES.iter().position(|s| s.sel == v) {
                                        active.set(i);
                                    }
                                })
                            />
                            <div class="gd-surf-note">
                                <code>{move || SURFACES[active.get()].sel}</code>
                                <span>{move || SURFACES[active.get()].note}</span>
                            </div>
                            <div class="gd-stage">
                                <div class="gd-stage__bg"></div>
                                <div class="gd-stage__row">{move || sample(active.get())}</div>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"实时参数"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">{rows}</div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"构建期参数（只读）"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <p class="c-caption" style="white-space:normal">
                                "这九项在构建期被烘进位移图与高光贴图，运行期改 CSS 变量没有效果。"
                                "要改：编辑 scripts/glass/glass-schema.js，重跑 build-optics 与 build-glass。"
                            </p>
                            {baked_rows}
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"输出"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <div class="l-row l-row--3">
                                <Button label="复制改动" variant=ButtonVariant::Primary on_click=copy_all />
                                <Button label="复制到全部面" variant=ButtonVariant::Tonal on_click=spread />
                                <Button label="恢复默认" variant=ButtonVariant::Plain on_click=reset />
                                <Switch
                                    checked=Signal::derive(move || full.get())
                                    on_change=Callback::new(move |v: bool| full.set(v))
                                />
                                <span class="c-caption">
                                    {move || if full.get() {
                                        "全部参数（可直接整段覆盖 optics.css）"
                                    } else {
                                        "只列出与初值不同的项"
                                    }}
                                </span>
                            </div>
                            <pre class="gd-tuner-out">{move || css_text.get()}</pre>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}

#[component]
fn KnobRow(
    sig: RwSignal<f64>,
    min: f64,
    max: f64,
    step: f64,
    var: &'static str,
    #[prop(optional)] unit: &'static str,
) -> impl IntoView {
    view! {
        <div class="gd-knob">
            <div class="gd-knob__track">
                <Slider
                    value=Signal::derive(move || sig.get())
                    min=min
                    max=max
                    step=step
                    label=var
                    on_input=Callback::new(move |v: f64| sig.set(v))
                />
            </div>
            <span class="gd-knob__value">
                {move || format!("{:.2}{}", sig.get(), unit)}
            </span>
            <span class="gd-knob__key">{var}</span>
        </div>
    }
}

/* ------------------------------- 样张 ------------------------------- */

fn sample(active: usize) -> AnyView {
    match active {
        0 => view! {
            <div class="c-glass-chrome gd-sample" style="width:280px;height:52px;border-radius:14px">
                <span class="gd-sample__name">"顶栏"</span>
            </div>
        }
        .into_any(),
        1 => view! {
            <div class="c-glass-panel gd-sample" style="width:240px;height:132px">
                <span class="gd-sample__name">"面板"</span>
            </div>
        }
        .into_any(),
        2 => view! {
            <Capsule size=CapsuleSize::Md>
                <span style="font-size:13px;font-weight:500;color:#fff">"Liquid Glass"</span>
            </Capsule>
        }
        .into_any(),
        3 => view! {
            <div class="c-dock__pill gd-sample" style="width:230px;height:46px;border-radius:23px">
                <span class="gd-sample__name">"停靠栏"</span>
            </div>
        }
        .into_any(),
        4 => view! {
            <div class="c-menu" role="menu" style="position:relative;left:0;top:0">
                <div class="c-menu__item" role="menuitem" tabindex="-1">"新建文件"</div>
                <div class="c-menu__item" role="menuitem" tabindex="-1">"复制一份"</div>
                <div class="c-menu__sep" role="separator"></div>
                <div class="c-menu__item c-menu__item--danger" role="menuitem" tabindex="-1">"删除"</div>
            </div>
        }
        .into_any(),
        5 => view! {
            <div class="c-toast" style="position:relative">
                <span>"已加入收藏"</span>
            </div>
        }
        .into_any(),
        6 => view! {
            <div class="c-glass-sheet gd-sample" style="width:240px;height:132px">
                <span class="gd-sample__name">"模态"</span>
            </div>
        }
        .into_any(),
        _ => view! {
            <div style="width:220px">
                <Slider value=Signal::derive(move || 46.0) min=0.0 max=100.0 step=1.0 label="拇指" />
            </div>
        }
        .into_any(),
    }
}

/* ------------------------------- 矩阵 ------------------------------- */

/// (当前值, 初值, 每面的构建期参数)。初值就是当前默认值，两者相同。
fn init_matrix() -> (Vec<f64>, Vec<f64>, Vec<Vec<(String, String)>>) {
    let mut vals = vec![f64::NAN; NS * NP];
    let mut baked = vec![Vec::new(); NS];

    let mut vars: Vec<&str> = KNOBS.iter().map(|k| k.var).collect();
    vars.extend(BAKED.iter().map(|(v, _)| *v));

    for (si, s) in SURFACES.iter().enumerate() {
        // 必须在元素还挂在文档里的时候把所有值抄成快照 ——
        // `getComputedStyle` 返回的是活对象，元素一移除，读到的全是空串。
        let Some(found) = probe(s.sel, &vars) else { continue };
        for (pi, k) in KNOBS.iter().enumerate() {
            if let Some(v) = found
                .iter()
                .find(|(name, _)| name == k.var)
                .and_then(|(_, raw)| parse_num(raw))
            {
                vals[si * NP + pi] = v;
            }
        }
        baked[si] = BAKED
            .iter()
            .map(|(var, _)| {
                let v = found
                    .iter()
                    .find(|(name, _)| name == var)
                    .map(|(_, raw)| raw.clone())
                    .unwrap_or_default();
                (var.to_string(), v)
            })
            .collect();
    }
    let base = vals.clone();
    (vals, base, baked)
}

/// 建一个视口外的探针元素，把 `vars` 的计算值抄成快照，然后移除。
///
/// 为什么必须"边挂边读"：`getComputedStyle()` 返回的是**活对象**，
/// 元素从文档里摘下来之后，它读到的每个属性都是空串。
fn probe(sel: &str, vars: &[&str]) -> Option<Vec<(String, String)>> {
    let doc = web_sys::window()?.document()?;
    let body = doc.body()?;
    let el = doc.create_element("div").ok()?;
    el.set_class_name(sel.trim_start_matches('.'));
    let style = el.dyn_ref::<web_sys::HtmlElement>()?.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("left", "-9999px");
    let _ = style.set_property("top", "0");
    body.append_child(&el).ok()?;
    let cs = web_sys::window()?.get_computed_style(&el).ok()??;
    let out: Vec<(String, String)> = vars
        .iter()
        .map(|v| {
            let raw = cs
                .get_property_value(v)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            ((*v).to_string(), raw)
        })
        .collect();
    let _ = body.remove_child(&el);
    Some(out)
}

fn parse_num(raw: &str) -> Option<f64> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    // 可能是 "1" / "0.55" / "24px" / "calc(24px)" —— 取头一个数字。
    let head: String = t
        .chars()
        .skip_while(|c| !(c.is_ascii_digit() || *c == '-' || *c == '.'))
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    head.parse::<f64>().ok()
}

fn build_css(vals: &[f64], base: &[f64], all: bool) -> String {
    let mut out = String::new();
    for (si, s) in SURFACES.iter().enumerate() {
        let mut block = String::new();
        for (pi, k) in KNOBS.iter().enumerate() {
            let cur = vals[si * NP + pi];
            let old = base[si * NP + pi];
            if cur.is_nan() {
                continue;
            }
            // 增量模式：只留和初值不同的项（覆盖默认值的最小差集）。
            // 全量模式：这一面没有的参数依然是 NaN，自然被跳过。
            if !all && (cur - old).abs() < 0.005 {
                continue;
            }
            block.push_str(&format!("  {}: {};\n", k.var, trim2(cur)));
        }
        if !block.is_empty() {
            out.push_str(&format!("{} {{\n{}}}\n", s.sel, block));
        }
    }
    if out.is_empty() {
        "/* 还没有改动 */".to_string()
    } else {
        out
    }
}

fn trim2(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if (r - r.round()).abs() < f64::EPSILON {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/* ------------------------------- 写 DOM ------------------------------- */

/// 建/更新一张专用样式表，放 head 末尾（排在 `#gd-styles` 之后）。
/// `!important` 是为了盖掉 `<html>` 上的行内变量。
fn apply_tuner_style(css: &str) {
    write_style("gd-tuner", css);
}

fn set_rule(sel: &str, decl: &str) {
    let id = format!("gd-tuner-{}", sel.trim_start_matches('.').replace("__", "-"));
    write_style(&id, &format!("{sel}{{{decl} !important}}"));
}

fn write_style(id: &str, css: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let el = match doc.get_element_by_id(id) {
        Some(el) => el,
        None => {
            let Ok(el) = doc.create_element("style") else { return };
            el.set_id(id);
            if let Some(head) = doc.head() {
                let _ = head.append_child(&el);
            }
            el
        }
    };
    if let Some(style) = el.dyn_ref::<web_sys::HtmlElement>() {
        style.set_text_content(Some(css));
    }
}

fn copy_text(text: &str) {
    let Some(win) = web_sys::window() else { return };
    let _ = win.navigator().clipboard().write_text(text);
}
