# GlassDesign

Tauri 2 + Rust 的桌面界面库。

库里只有一个可复制单元：`控件/`。整个文件夹拷进别的项目，加一行 path 依赖，调一次 `install()`，就能用 `view!` 拼界面；升级时用新文件夹覆盖旧文件夹。

> **状态：P2 基础控件已落地**（2026-11-15）。
> 51 个 Rust 组件、五个画廊分区、Tauri 2 宿主；玻璃材质为**构建期预计算贴图 + 静态 SVG filter**，运行时没有 JavaScript、canvas 或 WebGL。

---

## 和原仓库的关系

| 仓库 | 是什么 | 状态 |
|---|---|---|
| `Golden Gate HTML Design` | 原库：Electron + 原生 HTML/CSS/ES Module，84 个命令式工厂 | **冻结为参照**，不再演进 |
| `GlassDesign` | 新库：Tauri 2 + Rust + Leptos，声明式组件 + 静态 SVG 材质 | **P2 · 51 组件** |

旧库不是“上一版”，是**视觉与行为的对照标本**：10,204 行 CSS、8 套玻璃参数、766 条断言都留在旧仓库，新库逐批对照移植。

## 一句话结论

> 这不是把 76 个 JS 文件翻译成 Rust。
> 是把库切成五个平面：**表现面原样保留，材质面静态化，结构面与行为面重写，宿主面替换。**

- **表现面**（`styles/`，10,204 行）：与框架无关，51/62 个分区可逐控件切走。
- **材质面**：构建期用纯函数算好位移/高光贴图，生成静态 SVG filter；运行时只挂 CSS 类名，没有 JS 引擎。
- **结构面**（`scripts/ui/`，76 文件 18,664 行）：命令式工厂，全部改为 Rust 组件。
- **行为面**（层栈、选择、命令、撤销、测量、焦点）：逻辑可对照，代码重写。
- **宿主面**（Electron 6 个通道、透明窗口、红绿灯）：换成 Tauri 对等实现。

## 液态玻璃的当前做法

1. `scripts/build-glass.mjs` 用 `glass-schema / glass-shape / glass-optics` 算出每个固定形状的位移图与高光图。
2. 贴图编码成 PNG，内嵌进 `控件/assets/glass-defs.svg` 的 `<filter>` 里。
3. CSS 只写 `backdrop-filter: blur(...) saturate(...) url(#gd-glass-capsule-180)`。
4. `install()` 注入 CSS 与这份静态 SVG，不注入任何脚本。

**关键约束**：Chromium 的 `feImage` 不能可靠地把预计算贴图拉伸到任意尺寸。贴图必须和元素尺寸对齐，所以玻璃组件按**固定尺寸档**做：

| 档位 | 尺寸 | 用在哪 |
|---|---|---|
| `CapsuleSize::Round` | 34 × 34 | 顶栏圆形按钮 |
| `CapsuleSize::Sm` | 112 × 34 | 短胶囊 |
| `CapsuleSize::Md` | 180 × 34 | 长胶囊 |
| `CapsuleSize::Auto` | 自适应 | 不套 SVG filter，走纯 CSS 兜底 |

要加新尺寸，在 `scripts/build-glass.mjs` 的 `SHAPES` 里加一条，重新生成贴图与 CSS；不要指望一张图拉伸到所有宽度。

## 三条硬规矩

1. **`控件/` 是唯一复制单元。** 它只能依赖自己 `Cargo.toml` 里列出的 crates，不许 import 仓库里的 `应用/`、`scripts/`、`docs/`。
2. **外观只走令牌与类名。** Rust 里不出现字面色值、字号、圆角、阴影；一律用 `css/tokens.css` 的变量和既有类名。
3. **材质由固定尺寸档触发。** 组件只写 `class` / `data-size`；贴图与 filter 在构建期生成，运行时不计算。

## 目录

```text
GlassDesign/
├── README.md                  ← 你在这里
├── Cargo.toml                 workspace
├── .cargo/config.toml         清空代理（本机 cargo 需要）
├── docs/
│   ├── 00-探索-脉络.md         现状、判断、选型、风险、路线
│   ├── 01-控件契约.md          声明式 API：props / 受控 / 插槽 / 生命周期
│   ├── 02-迁移矩阵.md          84 个工厂 → Rust 组件的分批与难度
│   ├── 03-实现清单.md          51 个已实现组件、简化边界与验证记录
│   └── 04-交接.md             现在到哪一步、为什么、坑在哪、下一步
├── 控件/                      ← 唯一复制单元
│   ├── Cargo.toml
│   ├── src/                   Rust 组件
│   ├── css/                   tokens / optics / base / layout / components(64)
│   └── assets/glass-defs.svg  静态 SVG filter + 预计算贴图
├── 应用/
│   ├── ui/                    Leptos 画廊（wasm）
│   └── src-tauri/             Tauri 2 宿主
├── scripts/                   构建 CSS / 贴图 / 图标 / 截图
└── index.html                 静态视觉快照（P1）
```

## 快速开始

```sh
# 1. 生成 CSS、SVG filter、图标
node scripts/build-optics.mjs
node scripts/build-glass.mjs
node scripts/build-css.mjs
node scripts/gen-icons.mjs "../Golden Gate HTML Design/scripts/core/icons.js"

# 2. 编译画廊 wasm
cargo build -p glass-design-gallery --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir 应用/ui/dist \
  target/wasm32-unknown-unknown/debug/glass_design_gallery.wasm

# 3. 起静态服务看画廊
cd 应用/ui/dist && python -m http.server 8777

# 4. 导出静态快照（另开一个终端，在仓库根执行）
node scripts/build-static.mjs

# 5. Tauri 宿主
cargo build -p glass-design-app
```

`index.html` 是静态视觉快照，不含交互；交互版在 `应用/ui/dist/index.html`。

## 怎么读

- 第一次接触：本文件 + [`docs/00-探索-脉络.md`](docs/00-探索-脉络.md)。
- 要动手写控件：先读 [`docs/01-控件契约.md`](docs/01-控件契约.md)，再查 [`docs/02-迁移矩阵.md`](docs/02-迁移矩阵.md)。
- 要复制控件库到别的项目：读 [`控件/README.md`](控件/README.md)。
- 要接手继续做：读 [`docs/04-交接.md`](docs/04-交接.md)。

## 下一步

P2 已完成：Button / IconButton / Field / Switch / Segmented / Checkbox / RadioGroup / Slider / Stepper / NumberField / TextArea / SearchField / Select / FormField / Badge / Chip / Avatar / ProgressRing / Progress / Spinner / Skeleton / Empty / InlineMessage / List / Table / Disclosure / Tabs / Breadcrumb / Pagination / Dialog / Toast / Tile / MediaTile / NavInfo / NavCard / Capsule / AppShell / NavColumn / Toolbar。

P3 待做：RangeSlider、Combobox、DatePicker / TimePicker、FilePicker / DropZone、TokenField、ColorWell / Swatches、DataTable / TreeView / CollectionView、独立 Menu / Popover / Tooltip、Drawer / Sheet、Inspector / SplitView / StatusBar。

完整清单与简化边界见 [`docs/03-实现清单.md`](docs/03-实现清单.md)。
