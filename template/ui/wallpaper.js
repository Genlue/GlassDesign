/* --------------------------------------------------------------------------
   由 glass-design-host 提供（host/assets/wallpaper.js），应用不需要自己写这段。
   落地位置：应用/ui/dist/wallpaper.js，用普通 <script src="wallpaper.js"> 引入，
   **不是 ESM** —— 它必须在页面解析到这里时就跑，不等任何模块加载。

   宿主壳层与背景层接线：
     · 跑在 Tauri 里（window.__TAURI__ 存在）→ 在 <html> 上写 data-shell="host"，
       去掉浏览器预览用的 10px 外边距（.l-app inset: 0、方角、无投影）。
     · #wallpaper 始终使用内置合成渐变背景（data-source="demo" / --wallpaper），
       不再进行桌面截屏或挂载 WDA_EXCLUDEFROMCAPTURE，确保远程桌面、系统截图、
       录屏与任务栏预览稳定可用。
   -------------------------------------------------------------------------- */
const wp = document.getElementById('wallpaper');
if (wp && (!wp.dataset.source || wp.dataset.source === 'host')) {
  wp.dataset.source = 'demo';
}

if (window.__TAURI__) {
  // 宿主里 UI 就是窗口本身：去掉那圈给浏览器预览用的 10px 缝。
  // 写在 <html> 上而不是 .l-app 上 —— 属性立刻生效，Leptos 后挂载的
  // 节点一样命中，不用等 DOM 就绪。
  document.documentElement.dataset.shell = 'host';
}
