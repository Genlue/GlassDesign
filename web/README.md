# GlassDesign for JS hosts

`glass-web` mounts actual GlassDesign/Leptos components into HTML elements owned by a JS host. It does not bootstrap a Leptos application, change the document theme, or include application commands or state.

## Build on Windows

From the GlassDesign root:

```powershell
.\scripts\build-web.ps1 -OutDir 'C:\path\to\your\host\assets\glass'
```

The script uses the local `wasm32-unknown-unknown` Rust target, cached Cargo dependencies, the workspace `Cargo.lock`, and **wasm-bindgen CLI 0.2.130** on `PATH`. It builds offline. `-OutDir` can be absolute or relative to the caller. The release output is `glass_web.js` (ES module), `glass_web_bg.wasm`, `glass.css`, `glass-defs.svg`, and generated `.d.ts` declarations. Keep the JS and WASM files from the same build together. Add `-KeepSymbols` to preserve WASM function names for browser crash traces.

## Load and mount

Serve the assets over HTTP or the application's webview asset protocol; use `application/wasm` for the WASM file. Link the stylesheet once. It is the full GlassDesign stylesheet, including global resets, not scoped component CSS. Theme tokens are inherited; put `data-theme="dark"` on the document root to opt into the dark palette. For the glass filters, inline the contents of `glass-defs.svg` **once** in the same document (for example, at host build time); an `<img>` does not register same-document filter IDs.

```html
<link rel="stylesheet" href="./glass/glass.css">
<div id="search-host"></div>
<script type="module">
  import init, { mount_widget } from './glass/glass_web.js';
  await init();
  const widget = mount_widget(document.querySelector('#search-host'), 'search-field', {
    id: 'filter-query', label: 'Filter items', placeholder: 'Search…', width: 240,
  }, (value) => console.log(value));

  // When the host removes or replaces the widget:
  // widget.dispose();  // unmount; safe to call repeatedly before free()
  // widget.free();     // optional: release the wasm-bindgen wrapper immediately
</script>
```

`mount_widget(element, kind, props, emit) → WidgetHandle` mounts synchronously. Supply one current-document light-DOM `HTMLElement` per widget and an object for `props` (use `{}` if empty). The adapter appends to the host, and `dispose()` does not delete the host or its unrelated children. Keep the handle and dispose before replacing the host DOM, including nested widgets you no longer need. For controlled props, use `handle.update({ value: next })` (see below). To replace structural props such as labels or options, dispose and remount. Mutating the original JS props object has no effect. `free()` releases the wrapper and unmounts if not already disposed; do not call methods afterward.

Pass a function even if you don't need events (`() => {}`). The callback is delivered in a microtask, after the component's current handler returns, so it can safely dispose/remount. Pending emissions are canceled on disposal. Actions call `emit()` with zero arguments; value events call `emit(string)`. A callback exception is reported to the console. There is no `eval` or dynamic code generation in the adapter.

## Widget contract

Kinds and enum values are case-sensitive. `?` below means optional. `null`/`undefined` are treated as omitted for optional props. Recognized props with invalid types/enum values and unknown kinds throw; extra keys are ignored. `icon` is a built-in GlassDesign kebab-case key (e.g. `plus`, `folder-open`, `settings`); see `IconName` in the component library for the inventory. Empty optional icons are absent. Unknown keys and raw SVG are rejected. Keep app-specific icon aliases in the host. Button sizes are `sm`, `md` (default), `lg`, with `small`, `medium`, `large` aliases.

| Kind | Props | Event |
| --- | --- | --- |
| `button` | `label?`, `icon?`, `variant?` (`default`/`secondary`, `primary`, `ghost`/`plain`, `tonal`, `danger`), `size?`, `disabled?`, `className?` | `emit()` |
| `icon-button` | `label`, `icon`, `variant?` (`default`/`plain`/`ghost`, `tonal`, `on`), `size?`, `disabled?`, `active?`, `toggle?`, `className?`. `toggle` exposes `aria-pressed` from `active`; it does not change `active` on click. Use `variant: 'on'` for the native on appearance. | `emit()` |
| `nav-item` | `label?`, `icon?`, `selected?`, `count?` (nonnegative integer), `disabled?`. Supply the appropriate group/listbox context in the host. | `emit()` |
| `nav-card` | `title?`, `desc?`, `icon?`. Uses the solid interactive NavCard. | `emit()` |
| `segmented` | `label?`, `options: [{value, label, icon?}]`, `value?` (defaults to first option). | `emit(value)` |
| `search-field` | `value?`, `label?`, `placeholder?`, `width?` (integer pixels; omitted/0 = 180), `disabled?`, `id?` (on the native `.c-field__input`). | `emit(value)` on committed input changes; clear button or Escape clears, emits `""` when changed, and restores input focus. Intermediate IME input and composing Escape are ignored. |
| `select` | `options: [{value, label, disabled?}]`, `value?`, `label?`, `width?` (integer pixels; omitted/0 = 180). | `emit(value)` |
| `inline-message` | `tone?` (`info`, `success`, `warning`, `danger`/`error`), `title?`, `message?`. | None |
| `badge` | `label`, `variant?` (`neutral`, `accent`, `danger`), `icon?`. | None |
| `card` | `className?`; DOM slot `body` (also the default). | None |
| `status-bar` | `{}`; DOM slots `start` (default), `end`. | None |
| `toolbar` | `{}`; DOM slot `body`. This is the gallery's absolute-positioned toolbar, with empty regions pointer-transparent by default. Hosts own layout and native drag policy. | None |
| `capsule` | `size?`: `auto` (content width), `round` (34×34), `sm` (112×34), `md` (180×34); `className?`; DOM slot `body`. | None |
| `glass-panel` | `size?`: `auto` (content dimensions), `320x220`, `480x320`, `640x420`; DOM slot `body`. | None |
| `traffic-lights` | `focused?` (defaults true), `maximized?`, `dirty?`, `reveal?` (others default false). No native window calls are made by this component. | `emit('close' \| 'minimize' \| 'zoom')` |
| `split-grip` | `label`, `controls?` (DOM ID), `value?`, `min?` (default 0), `max?` (default 600), in whole pixels. The host positions the standalone `.c-resizer` and updates its pane width. | `emit(widthAsString)` for each user change |
| `menu` | `label?`, `icon?`, `className?`, `items: [{id, label, icon?, shortcut?, disabled?, danger?, checked?, separatorBefore?}]`. Uses GlassDesign's anchored `MenuButton`, with flat items. This is not a point/context menu API. | `emit(id)` |

Option values and menu item IDs are strings; use unique values within a widget. SearchField, Segmented and Select keep local state from the initial `value`, so input and selection work without remounting. `select` and `menu` share the document overlay stack and portal to `body`. Mount them outside native `<dialog>` elements (mount rejects a dialog ancestor), and do not move them into a dialog afterward. Use native selects/menus in dialogs. Disposing closes and removes the portal, even if it is open or awaiting positioning.

### Controlled updates and material

`handle.update(patch)` accepts these reactive keys only, does not recreate a DOM node, and does not emit a user event. Omitted/`null` fields are unchanged; invalid patches throw a `TypeError` without partially changing values.

- Segmented, Select: `value`.
- SearchField: `value`, `disabled`. Let the native input keep its draft during IME composition; set `value` when the host changes the query.
- TrafficLights: `focused`, `maximized`, `dirty`. No need to redraw on native focus/window-state events.
- SplitGrip: `value`, `min`, `max`. It uses pointer capture; Arrow Left/Right step 8px (Shift: 24px), Home/End reach the bounds. Keep the DOM connected during a drag and persist width separately in the host if needed.

Inline SVG definitions enable refraction and the optical highlight for fixed-size materials. Keep their border-box dimensions unchanged, and give them visible backdrop content to sample (not a flat opaque toolbar parent). Auto sizes and unavailable SVG filters retain the CSS coating and backdrop blur. Reduced motion disables transitions without disabling control updates.

### DOM slots

Place the original child nodes in the host **before mounting**. On each direct child, `data-glass-slot="body"` names the Card, Toolbar, Capsule, or GlassPanel body; `data-glass-slot="start"` or `"end"` names a StatusBar side. Unmarked children (including text nodes) go to the default slot. The adapter moves the nodes into the corresponding real component containers, preserving node identity and listeners. On disposal, nodes still in their projected slots are moved back to the host in source order. A node the host moved out of its slot or removed is not reclaimed. This is initial projection only, not live slot distribution or HTML-string injection.

## Verification

```powershell
cargo check -p glass-web --target wasm32-unknown-unknown --locked --offline
.\scripts\build-web.ps1 -OutDir target\glass-web-smoke
node web/smoke.mjs target/glass-web-smoke
$env:GD_REDUCED_MOTION='1'; node web/smoke.mjs target/glass-web-smoke
```

The browser smoke test uses local headless Microsoft Edge and Node with native `fetch` support. Pass a browser executable as a second argument when Edge is not installed in its default Windows location.
