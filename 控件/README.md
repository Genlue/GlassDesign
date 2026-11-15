# `控件/` · 唯一复制单元

这个文件夹整个拷进别的项目就能用；升级时用新文件夹覆盖它。

> **P1 骨架已落地。** 现在有 `Cargo.toml`、`src/`、`css/`、`assets/`，可以编译、可以拼画廊。
> 材质改为**构建期预计算贴图 + 静态 SVG filter**，运行时没有 JavaScript。

---

## 1. 为什么是一个文件夹

旧库的复制方式是“拷 `styles/` + 拷 `scripts/` + 改 import 路径”，三处散落，升级时容易漏。新库把它收成一个 crate：

```text
控件/
├── Cargo.toml            glass-design
├── README.md             本文件
├── src/                  Rust 组件
├── css/                  tokens / optics / base / layout / components(63)
└── assets/glass-defs.svg 预计算位移/高光贴图 + 静态 filter
```

样式与静态 SVG 都用 `include_str!` 编进 crate，运行时由 `install()` 注入。**复制一个文件夹就够，不需要再拷第二处资源。**

---

## 2. 三种用法

### 2.1 Rust / Tauri 项目（推荐）

```toml
# Cargo.toml
[dependencies]
glass-design = { path = "vendor/控件" }
```

```rust
fn main() {
    glass_design::install();
    // Tauri 启动 + Leptos mount
}
```

```rust
use glass_design::prelude::*;

view! {
    <Card>
        <SectionTitle>"设置"</SectionTitle>
        <Row label="启用">
            <Switch checked=Signal::derive(move || enabled.get()) />
        </Row>
        <Capsule size=CapsuleSize::Md>
            <span>"Liquid Glass"</span>
        </Capsule>
    </Card>
}
```

### 2.2 原生 HTML / JS 项目

拷 `控件/css` 与 `控件/assets`：

```html
<link rel="stylesheet" href="./控件/css/glass.css">
<!-- 放到 body 末尾：静态 filter 必须和页面在同一个文档里 -->
<div hidden>/* 把 glass-defs.svg 的内容内联在这里 */</div>
```

用同一套类名与 `data-size`，不需要 Rust，也不需要脚本。适合 Lumina 这类现有 Tauri + 原生 JS 项目。

注意：`glass-defs.svg` 必须内联，不能 `<img src>`；`url(#id)` 解析不到外部 SVG 文档里的 filter。

### 2.3 旧 JS 项目

继续用 `Golden Gate HTML Design`。旧库冻结，不再加控件。

---

## 3. 复制

1. 把整个 `控件/` 拷到目标项目，例如 `vendor/控件`。
2. `Cargo.toml` 加 path 依赖。
3. `main()` 调一次 `install()`。

没有第四步。不需要改 `tauri.conf.json` 的资源表，不需要额外 `<link>`，不需要构建脚本。

---

## 4. 替换（升级）

1. 用新版本覆盖 `vendor/控件/`。
2. `cargo build`（path 依赖会自动重编）。
3. 看 `控件/变更.md`：
   - 同 major：应当零改动。
   - major：按条目改调用方代码。

`install()` 会把版本写到 `<html data-gd-version="x.y.z">`，页面页脚可显示，截图即证据。

---

## 5. 硬规矩

1. **不许 import 仓库里的 `应用/`、`scripts/`、`docs/`。** 这个文件夹必须能单独打包。
2. **不许依赖运行期文件路径。** 样式、贴图、图标一律 `include_str!`。
3. **不许出现第三个全局单例。** 只有 `OverlayHost` 与 `Theme` 两个 `thread_local`。
4. **不许在 Rust 里写字面色值/px。** 新值先加到 `css/tokens.css`。
5. **不许把 `leptos::` 类型暴露给调用方**（`Signal`/`Callback` 除外）。
6. **不许改 DOM 契约而不改 major。** 类名、`data-*`、ARIA、键盘行为都算契约。
7. **不许用一张贴图通吃所有尺寸。** 玻璃尺寸必须落在 `scripts/build-glass.mjs` 的档位表里；否则走 `Auto` 的纯 CSS 兜底。

---

## 6. 玻璃尺寸档

| 档位 | 尺寸 | CSS | 说明 |
|---|---|---|---|
| `CapsuleSize::Round` | 34 × 34 | `.c-capsule--round` | 顶栏圆钮 |
| `CapsuleSize::Sm` | 112 × 34 | `data-size="112"` | 短胶囊 |
| `CapsuleSize::Md` | 180 × 34 | `data-size="180"` | 长胶囊 |
| `CapsuleSize::Auto` | 自适应 | 无 | 不套 SVG filter，只走 CSS |

原因：Chromium 的 `feImage` 不能可靠拉伸预计算贴图；贴图尺寸必须与元素一致。加档位的流程见 [`../docs/01-控件契约.md`](../docs/01-控件契约.md) §12.1。

---

## 7. 版本

- `Cargo.toml` 的 `version` 与 `src/lib.rs` 的 `VERSION` 必须一致。
- 新增 prop / 新增组件：minor。
- 改 props 形状、删组件、改 DOM/键盘契约、改贴图尺寸：major。
- 不承诺跨 major 的样式兼容；CSS 变量改名先加别名，下一个 major 再删。

---

## 8. 当前状态

| 项 | 状态 |
|---|---|
| 目录契约 | 已定（本文件） |
| Rust 组件 | P1：Button / IconButton / Field / Switch / Card / Row / SectionTitle / Divider / AppShell / Main / ScrollArea / NavColumn / NavGroup / NavSection / NavItem / Toolbar / ToolbarLead / ToolbarPills / ToolbarTitle / Capsule |
| CSS 迁移 | tokens / optics / base / layout + 63 个分区已进 `css/src`；`24-glass-renderer` 停用 |
| 静态材质 | 胶囊 34/112/180 + 滑块拇指；构建期贴图，运行时无 JS |
| 示例应用 | 画廊跑通（浅色/深色、导航、主题切换） |
| Tauri 宿主 | 可编译；透明窗口/无边框待实机截图 |
| `index.html` | P1 静态视觉快照已生成 |

排期与出口条件见 [`../docs/02-迁移矩阵.md`](../docs/02-迁移矩阵.md) §2 与 [`../docs/00-探索-脉络.md`](../docs/00-探索-脉络.md) §13。
