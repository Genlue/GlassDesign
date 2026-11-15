// 从旧库 icons.js 生成 控件/src/base/icon.rs。
// 用法：node scripts/gen-icons.mjs <旧库路径>
import { readFileSync, writeFileSync } from 'node:fs';

const src = process.argv[2] ?? '../../Golden Gate HTML Design/scripts/core/icons.js';
const text = readFileSync(src, 'utf8');
const table = text.match(/const PATHS = \{([\s\S]*?)\n\};/);
if (!table) throw new Error('找不到 PATHS 表');

const entries = [];
const re = /'([^']+)':\s*\n?\s*'((?:[^'\\]|\\.)*)'/g;
let m;
while ((m = re.exec(table[1]))) entries.push({ name: m[1], path: m[2] });

if (entries.length !== 97) console.warn(`图标数量 ${entries.length}，预期 97`);

const constName = (name) =>
  name.toUpperCase().replace(/[^A-Z0-9]+/g, '_').replace(/^(\d)/, '_$1');

const seen = new Set();
for (const e of entries) {
  let c = constName(e.name);
  if (seen.has(c)) c += '_2';
  seen.add(c);
  e.constName = c;
}

const consts = entries
  .map((e) => `    pub const ${e.constName}: Self = Self("${e.name}");`)
  .join('\n');

const arms = entries
  .map((e) => `        "${e.name}" => r#"${e.path}"#,`)
  .join('\n');

const out = `// 由 scripts/gen-icons.mjs 生成，请勿手改。
// 来源：Golden Gate HTML Design/scripts/core/icons.js（97 个图标）

use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IconName(pub(crate) &'static str);

impl IconName {
${consts}
}

impl IconName {
    pub fn path(self) -> &'static str {
        match self.0 {
${arms}
            _ => "",
        }
    }
}

pub const ICON_COUNT: usize = ${entries.length};

/// 行内 SVG 图标。24 视框，线宽由 CSS 的 --ic-stroke 控制。
#[component]
pub fn Icon(
    #[prop(into)] name: IconName,
    #[prop(default = 16)] size: u16,
) -> impl IntoView {
    view! {
        <svg
            class="ic"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
            inner_html=name.path()
        ></svg>
    }
}
`;

writeFileSync(new URL('../控件/src/base/icon.rs', import.meta.url), out, 'utf8');
console.log(`icon.rs · ${entries.length} 个图标`);
