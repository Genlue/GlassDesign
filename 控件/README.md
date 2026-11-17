# glass-design

Tauri 2 + Leptos 0.8（CSR）的桌面控件库。104 个公开组件，一个 `prelude` 出口。

样式（331 KB CSS）与玻璃贴图（134 KB SVG）用 `include_str!` 编进 wasm，由 `install()` 注入。不需要构建脚本，不需要额外拷贝资源目录。

## 1. 一行接入

```sh
cargo add glass-design --git https://github.com/Genlue/GlassDesign --tag v0.8.0
cargo add leptos --features csr
cargo add wasm-bindgen
```

```rust
use leptos::prelude::*;               // view! 与 #[component]
use glass_design::prelude::*;         // 组件 + bootstrap

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    bootstrap(|| view! { <Card><SectionTitle>"你好"</SectionTitle></Card> });
}
```

`bootstrap` 已经把 `install()`、`Theme::provide()`、`OverlayHost` 和挂载一起做完。页面里不用再写 `<link>`，不用改 `tauri.conf.json`。

三个依赖都得写。`#[component]` / `view!` 展开成 `::leptos::…`，`#[wasm_bindgen]` 展开成 `::wasm_bindgen::…`——都是绝对路径，指向**你这一层** crate 根的外部 crate。re-export 只多给一个 `glass_design::leptos` 路径，改不了宏里写死的 `::leptos`。

re-export 仍然有用：`Signal` / `Callback` 这类 props 类型需要版本锚点，`glass_design::leptos` 在泛型与文档里也能直接引用。它不替代声明。

应用里写 `leptos = "0.8"` 就行，别写 `=0.8.x` 精确锁：库用的是 caret，你锁死就会各拿一份 leptos，`Signal` 变成两个类型。

## 2. 四种用法

下面三段只演示 `glass-design` 这一行怎么写；`leptos` 与 `wasm-bindgen` 按 §1 一起加，三种方式都一样。

**当前未发布到 crates.io**：自己的项目用 2.2 的 git tag 即可，实测能从仓库里解析到 `控件/` 这个 workspace 成员。2.1 只是留作将来对外发布的写法。完整接入步骤见仓库 [`docs/05-接入手册.md`](../docs/05-接入手册.md)。

### 2.1 crates.io

```toml
[dependencies]
glass-design = "0.8"
# + leptos = { version = "0.8", features = ["csr"] }
# + wasm-bindgen = "0.2"        ← 都是宏展开需要的，见 §1
```

### 2.2 git（按 tag 锁）

```toml
[dependencies]
glass-design = { git = "https://github.com/<owner>/GlassDesign", tag = "v0.8.0" }
# + leptos / wasm-bindgen，见 §1
```

tag 比分支可靠：分支会动，tag 不会。

已实测：`tag = "v0.8.0"` 能被 cargo 直接解析到仓库里的 `控件/`（workspace 成员），不需要发布到 crates.io。自己的项目之间互相引用，这条路就够了；crates.io 只在要给别人用时才有意义。

### 2.3 path（vendor 进仓库）

```toml
[dependencies]
glass-design = { path = "vendor/glass-design" }
# + leptos / wasm-bindgen，见 §1
```

适合内网或不方便拉 crates.io 的项目。把发布包解开放进 `vendor/` 即可，`src/` + `css/glass.css` + `assets/glass-defs.svg` 就够。

### 2.4 纯 HTML + CSS（不用 Rust）

只想要外观时，引两份文件：

```html
<link rel="stylesheet" href="./glass.css">
<!-- 放进 body：静态 filter 必须和页面在同一个文档 -->
<div hidden aria-hidden="true"><!-- glass-defs.svg 的内容原样贴进来 --></div>
```

类名与 `data-size` 和 Rust 侧一致，不需要脚本。

**`glass-defs.svg` 必须内联**，不能 `<img src>`：`url(#id)` 解析不到外部 SVG 文档里的 filter。

## 3. 两层入口

| 你想要 | 用哪个 | 它做什么 |
|---|---|---|
| 什么都别管 | `bootstrap(App)` | `install()` + `Theme::provide()` + `OverlayHost` + `mount_to_body` |
| 换主题 / 语言 / 不注入样式 | `bootstrap_with(App, BootstrapOptions { .. })` | 同上，参数可调 |
| 自己的根节点里要放 `<GlassApp>` | `GlassApp` | 组件形式的同一层：注入 + 提供上下文 + 渲染 children |
| 完全自己接管 | `install()` + `Theme::provide()` + `mount_to_body` | 三步自己排 |

```rust
pub struct BootstrapOptions {
    pub install: bool,          // false：样式与贴图不注入，自己 <link>
    pub theme: ThemeMode,       // Light / Dark / System
    pub accent: &'static str,   // 强调色，写进 --accent
    pub locale: Locale,         // 文案语言
}
```

`GlassApp` 与 `bootstrap` 的差别只有一处：`GlassApp` 要的是 `Children`，`bootstrap` 要的是返回 `IntoView` 的闭包。根组件捕获了 `Rc` 这类非 `Send` 值时用 `bootstrap`——`Children` 那一槽在 0.8 要求 `Send`。

## 4. 样式怎么进页面

默认开 `embedded` feature：CSS 与 SVG 编进 wasm，`install()` 建一个 `<style id="gd-styles">` 塞进 `<head>`，再建 `<div id="gd-glass-defs">` 塞进 `<body>`。幂等，重复调用不会重复插入。

关掉它：

```toml
[dependencies]
glass-design = { version = "0.8", default-features = false }
```

这时 `STYLES` / `GLASS_DEFS` 是空字符串，`install()` 什么都不注入。前端自己来：

```html
<link rel="stylesheet" href="/glass.css">
<script type="module">
  const defs = await (await fetch('/glass-defs.svg')).text();
  document.body.insertAdjacentHTML('beforeend', `<div hidden>${defs}</div>`);
</script>
```

两处文件从 crate 里取。构建期落盘，二选一：

```rust
// 开了 embedded：直接问库要
std::fs::write("dist/glass.css", glass_design::styles_css()).unwrap();
std::fs::write("dist/glass-defs.svg", glass_design::glass_defs_svg()).unwrap();
```

```sh
# 关掉 embedded：发布包里就有这两份，拷出来即可
cp vendor/glass-design/css/glass.css dist/
cp vendor/glass-design/assets/glass-defs.svg dist/
```

`styles_css()` / `glass_defs_svg()` 返回 `&'static str`。

收益：wasm 少 465 KB，首帧不用等 wasm 下载与实例化就能上样式。代价：多两个静态文件，且 `glass-defs.svg` 必须内联进文档。

## 5. 扩展点

### 5.1 自定义图标

`IconName` 是常量表，内置 97 个。没有的图标用 `custom()`，给 24 视框里的 path 片段：

```rust
const LOGO: IconName = IconName::custom(
    "M4 12a8 8 0 1 1 16 0 8 8 0 0 1-16 0Zm3 0h10M12 7v10",
);

view! { <Icon name=LOGO size=20 /> }
```

参数必须 `'static`：编译期就固定，运行期没有字符串查表。线宽由 `--ic-stroke` 控制，不要在 path 里写 `stroke-width`。

### 5.2 主题令牌覆盖

`Theme::set_token` 直接写 `<html>` 上的 CSS 变量，覆盖任意 `--*` 令牌：

```rust
let theme = Theme::provide();
theme.set_accent("#6e56cf");              // 换强调色
theme.set_token("--r-md", "10px");        // 中圆角改小
theme.remove_token("--r-md");             // 撤掉，回到 tokens.css 的值
```

强调色走 `set_accent`：`Theme::apply()` 每次主题切换都会重写 `--accent` / `--accent-hover` / `--accent-soft` / `--accent-ring`，用 `set_token` 改单个变量会被冲掉，hover 与 ring 也还停在旧色。`set_token` 只用于 `apply()` 不管的令牌：圆角、字号、字体、间距。

令牌清单见 `css/src/tokens.css`。不要覆盖组件内部的计算变量（如 `--thumb-s`），那是组件自己写的。

### 5.3 布局原语

十个组件包住 `.l-*` 空间类，页面里不写 `display:flex` / `gap` / `padding`：

| 组件 | 类 | 用途 |
|---|---|---|
| `Stack` | `l-stack` | 纵向堆叠；`fill` 在父级 flex 里吃掉剩余空间 |
| `Cluster` | `l-row` | 横排、垂直居中；`between` 两端顶开，`end` 靠右 |
| `Grid` | `l-grid` | 网格，列与间隙走令牌 |
| `Page` | `l-page` | 整页容器，自带裁剪与滚动边界 |
| `PageInset` | `l-page-inset` | 给被顶栏压住的那一行让位（与 `Stage` 二选一） |
| `Stage` | `l-stage` | 主内容舞台，绝对定位铺满并处理过度滚动；`flush` 撤掉顶部留白 |
| `Center` | `l-center` | 横纵居中（空态、加载态） |
| `Push` | `l-push` | 撑开的空节点，把后面的东西推右 |
| `Fill` | `l-fill` | 吃掉剩余空间，且 `min-width: 0` |
| `Pad` | `l-pad` | 内容边距；`wide` 是更松的一档 |

```rust
view! {
    <Page>
        <Pad>
            <Stack>
                <Cluster between=true>
                    <SectionTitle>"设置"</SectionTitle>
                    <Button label="保存" />
                </Cluster>
                <Divider />
            </Stack>
        </Pad>
    </Page>
}
```

## 6. 玻璃尺寸档

| 档位 | 尺寸 | 用在哪 |
|---|---|---|
| `CapsuleSize::Round` | 34 × 34 | 只装一枚图标按钮 |
| `CapsuleSize::Sm` | 112 × 34 | 定宽短胶囊 |
| `CapsuleSize::Md` | 180 × 34 | 定宽长胶囊 |
| `CapsuleSize::Auto` | 自适应 | 宽度跟内容走，纯 CSS 涂层（无位移折射） |
| 滑块拇指 | 24 × 16 | `Slider` / `RangeSlider` |
| `GlassPanelSize` | 320×220 / 480×320 / 640×420 | `GlassPanel` |
| 菜单 | 240×200 / 240×280 | `MenuButton fixed_size` |
| `SheetSize::Md` | 520×360 | `Sheet` |
| Toast | 320×34 | `ToastHost` |

原因：Chromium 的 `feImage` 不能可靠拉伸预计算贴图，贴图尺寸必须与元素尺寸一致。宽度不固定的用 `CapsuleSize::Auto`，塞进固定档位会露出比内容宽一截的外壳。

## 7. 版本与语义化

- `Cargo.toml` 的 `version` 与 `install()` 写出的 `data-gd-version` 同源（`env!("CARGO_PKG_VERSION")`，不会漂）。
- 新增 prop / 新增组件：minor。
- 改 props 形状、删组件、改 DOM / 键盘契约、改贴图尺寸：major。
- CSS 变量改名先加别名，下一个 major 再删。

## 8. 边界

104 个公开组件（`#[component]` 共 106 处，其中 `ParentScope` 是 `pub(crate)`、`Light` 是私有内部件，不计入公开面），全部从 `prelude` 导出，分八类：布局与表面 / 控件 / 表单 / 外观 / 数据与内容 / 导航 / 浮层与面板 / 状态与占位。

用之前先看 `docs/02-组件契约.md` §13 的边界表。几个常被问到的：

- `TreeView` 只有单选，没有多选与 typeahead。
- `SplitView` 只支持两栏，比例制（`--split` 0..1），不持久化。
- `Form` 没有字段注册表：字段不会自动跟随表单禁用。
- `DataTable` 行高必须固定（虚拟滚动的前提），不支持列可见性。
- `Sheet` 只有 `Md` 命中贴图，`Sm` / `Lg` 走纯 CSS。
- `Dialog` 的 `dismissible=false` 时 Esc 与遮罩都关不掉，这是刻意的。

这个 crate 只承诺 Windows / WebView2；macOS 与 Linux 是「能编译、样式有兜底」，玻璃折射不保证。
