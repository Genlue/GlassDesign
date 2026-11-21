import init, { mount_widget } from '/glass_web.js';

Error.stackTraceLimit = 40;
const errors = [];
const logError = console.error.bind(console);
console.error = (...args) => {
  logError(...args);
  errors.push(args.map((arg) => arg?.stack ?? String(arg)).join(' '));
};
window.addEventListener('error', (event) => errors.push(event.error?.stack ?? event.message));
window.addEventListener('unhandledrejection', (event) => errors.push(String(event.reason)));
const body = document.querySelector('main');
const target = (parent = body) => parent.appendChild(document.createElement('div'));
const noop = () => {};
const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
const tick = async () => { await frame(); await frame(); };
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const key = (el, key, props = {}) => el.dispatchEvent(new KeyboardEvent('keydown', {
  key, bubbles: true, cancelable: true, ...props,
}));
const query = (host, selector) => host.querySelector(selector);
let checks = 0;
function assert(condition, label) {
  checks++;
  if (!condition) throw new Error(label);
}
function throws(fn, label) {
  try { fn(); } catch (error) { assert(error instanceof TypeError, label); return; }
  assert(false, label);
}
function unmount(handle) { handle.dispose(); handle.dispose(); handle.free(); }

async function smoke() {
  await init();
  assert(document.querySelector('svg defs filter'), 'SVG filter definitions must be present');
  const events = [];
  const emit = (...args) => events.push(args);
  const actions = target();
  const button = mount_widget(actions, 'button', {
    label: '<em>Add</em>', icon: 'plus', variant: 'primary', size: 'lg', className: 'host-btn',
  }, emit);
  const nativeButton = query(actions, 'button.c-btn--primary.c-btn--lg.host-btn');
  assert(nativeButton?.textContent === '<em>Add</em>' && query(nativeButton, 'svg path'), 'actual Button and safe text');
  throws(() => mount_widget(actions, 'badge', {label: 'X'}, noop), 'duplicate mount rejected');
  nativeButton.click();
  assert(events.length === 0, 'events are queued');
  await Promise.resolve();
  assert(events.shift()?.length === 0, 'actions emit zero args');
  nativeButton.click();
  unmount(button);
  await Promise.resolve();
  assert(!actions.firstChild && !events.length, 'dispose removes view and cancels queued callback');
  const keep = actions.appendChild(document.createTextNode('preserve'));
  const disabled = mount_widget(actions, 'button', { label: 'Disabled', variant: 'ghost', disabled: true }, emit);
  assert(query(actions, 'button.c-btn--plain').disabled, 'ghost and disabled');
  query(actions, 'button').click();
  await Promise.resolve();
  assert(!events.length, 'disabled button does not emit');
  unmount(disabled);
  assert(actions.firstChild === keep && actions.childNodes.length === 1, 'unrelated child left alone');
  const icon = mount_widget(actions, 'icon-button', { label: 'Pin', icon: 'pin', variant: 'on', active: true, toggle: true }, noop);
  assert(query(actions, '.c-iconbtn--on[aria-pressed="true"][aria-label="Pin"] svg'), 'icon-button toggle state');
  unmount(icon);
  throws(() => mount_widget(actions, 'icon-button', { label: 'Bad', icon: '<svg></svg>' }, noop), 'rejects markup icon');
  throws(() => mount_widget(actions, 'unknown', {}, noop), 'rejects unknown kind');
  throws(() => mount_widget(actions, 'search-field', { width: '240' }, noop), 'rejects invalid prop types');

  const nav = target();
  const navItem = mount_widget(nav, 'nav-item', { label: 'Work', selected: true, count: 0, disabled: true }, emit);
  const item = query(nav, '[aria-selected="true"][aria-disabled="true"]');
  assert(query(item, '.c-nav__count').textContent === '0', 'NavItem count and disabled');
  key(item, 'Enter');
  await Promise.resolve();
  assert(!events.length, 'disabled NavItem does not emit');
  unmount(navItem);
  const navCard = mount_widget(nav, 'nav-card', { title: 'Work', desc: 'Details', icon: 'folder' }, emit);
  assert(query(nav, '.c-navcard--solid .c-navcard__desc')?.textContent === 'Details', 'interactive NavCard uses solid mode');
  key(query(nav, '[role=button]'), 'Enter');
  await Promise.resolve();
  assert(events.shift()?.length === 0, 'NavCard keyboard event');
  unmount(navCard);

  const seg = target();
  const segmented = mount_widget(seg, 'segmented', { label: 'View', options: [
    { value: 'a', label: 'A', icon: 'list' }, { value: 'b', label: 'B' },
  ] }, emit);
  assert(query(seg, '[aria-label="View"] [data-value=a][aria-checked=true]'), 'Segmented group and default');
  query(seg, '[data-value=b]').click();
  await tick();
  const selected = query(seg, '[data-value=b][aria-checked=true]');
  assert(events.shift()?.[0] === 'b' && selected, 'Segmented local selection');
  const track = query(seg, '.c-seg');
  const indicator = query(track, '.c-seg__ind');
  track.style.width = '280px';
  track.style.transform = 'scale(.8)';
  await tick();
  const outerWidth = track.offsetWidth;
  const previousWidth = selected.offsetWidth;
  selected.style.paddingInline = '42px';
  await tick();
  assert(track.offsetWidth === outerWidth && selected.offsetWidth > previousWidth
    && parseFloat(indicator.style.width) === selected.offsetWidth
    && indicator.style.transform === `translate(${selected.offsetLeft}px, ${selected.offsetTop}px)`,
    'Segmented remeasures option geometry in a fixed-width, scaled host');

  await wait(300);
  segmented.update({ value: 'a' });
  await tick();
  assert(query(track, '.c-seg__ind') === indicator && query(track, '[aria-checked=true]').dataset.value === 'a'
    && !events.length, 'controlled Segmented update preserves indicator and does not emit');
  const transformMs = parseFloat(getComputedStyle(indicator).transitionDuration);
  assert(matchMedia('(prefers-reduced-motion: reduce)').matches
    ? transformMs < 0.01 : indicator.getAnimations().some(animation => animation.transitionProperty === 'transform'),
    'real slide transition, suppressed when reduced motion is selected');
  const firstOption = query(track, '[data-value=a]');
  firstOption.focus();
  key(firstOption, 'ArrowRight');
  await tick();
  assert(document.activeElement === selected && events.shift()?.[0] === 'b', 'Segmented keyboard moves focus and selection');
  throws(() => segmented.update({ label: 'Static label' }), 'reject patch to static prop');
  // A change queues a geometry frame; unmount in the same turn as the JS emission.
  let fastSegmented;
  const fastSegHost = target();
  fastSegmented = mount_widget(fastSegHost, 'segmented', { options: [
    { value: 'a', label: 'A' }, { value: 'b', label: 'B' },
  ] }, () => unmount(fastSegmented));
  await tick();
  query(fastSegHost, '[data-value=b]').click();
  await tick();
  assert(!fastSegHost.firstChild, 'dispose Segmented from callback with geometry pending');
  segmented.dispose();
  throws(() => segmented.update({ value: "b" }), "reject update after disposal");
  segmented.free();
  window.dispatchEvent(new Event('resize'));

  const search = target();
  const searchProps = { id: 'search-input', value: 'initial', label: 'Query', placeholder: 'Filter', width: 240 };
  const searchHandle = mount_widget(search, 'search-field', searchProps, emit);
  const input = query(search, 'input.c-field__input#search-input');
  assert(input?.value === 'initial' && input.placeholder === 'Filter', 'SearchField native input id and value');
  function inputEvent(value, isComposing = false) {
    input.value = value;
    input.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing }));
  }
  await tick();
  inputEvent('n', true);
  key(input, 'Escape', { isComposing: true });
  await tick();
  assert(!events.length && input.value === 'n', `IME draft and Escape do not emit or clear (value=${input.value}, events=${JSON.stringify(events)})`);
  input.value = '你';
  input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: '你' }));
  inputEvent('你');
  await tick();
  assert(events.length === 1 && events.shift()?.[0] === '你', 'IME commit emits once');
  query(search, 'button').click();
  await tick();
  assert(events.shift()?.[0] === '' && input.value === '' && document.activeElement === input, 'clear button clears and focuses');
  inputEvent('sample');
  await tick();
  assert(events.shift()?.[0] === 'sample' && query(search, 'button'), 'normal input and reactive clear affordance');
  key(input, 'Escape');
  await tick();
  assert(events.shift()?.[0] === '' && input.value === '', 'Escape clears');
  unmount(searchHandle);
  const disabledSearch = mount_widget(search, 'search-field', { ...searchProps, disabled: true }, emit);
  assert(query(search, 'input').disabled && query(search, 'button').disabled, 'disabled SearchField input and clear');
  unmount(disabledSearch);

  const label = target();
  const message = mount_widget(label, 'inline-message', { tone: 'error', title: 'Error', message: 'Retry' }, noop);
  assert(query(label, '.c-inline--danger[role=alert] .c-inline__desc')?.textContent === 'Retry', 'actual InlineMessage');
  unmount(message);
  const badge = mount_widget(label, 'badge', { label: 'New', variant: 'accent' }, noop);
  assert(query(label, '.c-badge--accent')?.textContent === 'New', 'actual Badge');
  unmount(badge);

  const cardHost = target();
  const original = target(cardHost);
  original.dataset.glassSlot = 'body';
  const card = mount_widget(cardHost, 'card', { className: 'host-card' }, noop);
  assert(query(cardHost, 'section.c-card.host-card').firstElementChild === original, 'Card node identity');
  unmount(card);
  assert(cardHost.firstElementChild === original && cardHost.children.length === 1, 'Card reclaims body nodes');
  const statusHost = target();
  const end = target(statusHost);
  end.dataset.glassSlot = 'end';
  const start = target(statusHost);
  const status = mount_widget(statusHost, 'status-bar', {}, noop);
  assert(query(statusHost, '.c-statusbar__items-end').firstElementChild === end
    && query(statusHost, '.c-statusbar__items-start').firstElementChild === start, 'StatusBar named and default slots');
  unmount(status);
  assert(statusHost.children[0] === end && statusHost.children[1] === start, 'slots restore original order');

  const selectHost = target();
  const selectProps = { label: 'Project', width: 210, value: 'a', options: [
    { value: 'a', label: 'Alpha' }, { value: 'b', label: 'Beta' }, { value: 'c', label: 'Disabled', disabled: true },
  ] };
  const dialog = body.appendChild(document.createElement('dialog'));
  throws(() => mount_widget(target(dialog), 'select', selectProps, noop), 'native dialog select is unsupported');
  dialog.remove();
  const portalCount = () => [...document.body.children].filter((el) => el.tagName === 'DIV').length;
  const portalsBefore = portalCount();
  const select = mount_widget(selectHost, 'select', selectProps, emit);
  const selectButton = query(selectHost, '.c-select button');
  selectButton.click();
  await tick();
  const selectMenu = query(document, '.c-menu[data-state=open]');
  assert(selectMenu && !selectHost.contains(selectMenu) && getComputedStyle(selectMenu).visibility === 'visible'
    && Math.abs(selectMenu.getBoundingClientRect().left - selectButton.getBoundingClientRect().left) < 20,
    'Select portal mounts, positions and is visible');
  query(selectMenu, '[aria-disabled=true]').click();
  await Promise.resolve();
  assert(!events.length && selectButton.getAttribute('aria-expanded') === 'true', 'disabled select option');
  query(selectMenu, '[data-label=Beta]').click();
  await tick();
  assert(events.shift()?.[0] === 'b' && selectButton.textContent.includes('Beta')
    && selectButton.getAttribute('aria-expanded') === 'false', 'Select emits, updates and closes');
  selectButton.click();
  await tick();
  key(selectButton, 'Escape');
  await tick();
  assert(selectButton.getAttribute('aria-expanded') === 'false', 'Select Escape dismisses');
  selectButton.click();
  await tick();
  assert(selectButton.getAttribute('aria-expanded') === 'true', 'Select can reopen');
  const anotherHost = target();
  const anotherSelect = mount_widget(anotherHost, 'select', selectProps, noop);
  query(anotherHost, 'button').click();
  await tick();
  assert(selectButton.getAttribute('aria-expanded') === 'false'
    && query(anotherHost, 'button').getAttribute('aria-expanded') === 'true'
    && document.querySelectorAll('.c-menu[data-state=open]').length === 1,
    'independently mounted Selects share one stack');
  unmount(anotherSelect);
  selectButton.click();
  await tick();
  unmount(select);
  window.dispatchEvent(new Event('resize'));
  await tick();
  assert(!query(document, '.c-menu') && portalCount() === portalsBefore, 'dispose removes open Select including both portals');
  const rushed = mount_widget(selectHost, 'select', selectProps, emit);
  query(selectHost, 'button').click();
  // Flush reactive work but dispose before requestAnimationFrame placement.
  await Promise.resolve();
  unmount(rushed);
  await tick();
  assert(!query(document, '.c-menu') && portalCount() === portalsBefore, 'dispose pending Select placement');

  const menuHost = target();
  let menu;
  menu = mount_widget(menuHost, 'menu', { label: 'Actions', items: [
    { id: 'copy', label: 'Copy', icon: 'copy' }, { id: 'trash', label: 'Trash', disabled: true },
  ] }, (id) => { emit(id); unmount(menu); });
  query(menuHost, 'button').click();
  await tick();
  const copy = query(document, '.c-menu [data-id=copy]');
  assert(copy && document.activeElement === copy, 'MenuButton popover and keyboard focus');
  key(copy, 'Enter');
  await wait(175);
  assert(events.shift()?.[0] === 'copy' && !menuHost.firstElementChild && portalCount() === portalsBefore,
    `MenuButton picks item, callback can dispose, transition cleanup (events=${JSON.stringify(events)}, host=${menuHost.innerHTML}, portals=${portalCount()}/${portalsBefore})`);

  const lightsHost = target();
  const lights = mount_widget(lightsHost, 'traffic-lights', { focused: true }, emit);
  const lightButtons = [...lightsHost.querySelectorAll('button')];
  const zoom = lightButtons[2];
  const glyph = query(zoom, 'path'), initialGlyph = glyph.getAttribute('d');
  zoom.focus();
  await wait(150);
  assert(getComputedStyle(glyph.parentElement).opacity === '1', 'TrafficLights reveals all glyphs on keyboard focus');
  lights.update({ focused: false, maximized: true, dirty: true });
  await wait(150);
  assert(zoom === lightsHost.querySelector('button:last-child') && document.activeElement === zoom
    && zoom.ariaLabel === '还原窗口' && glyph.getAttribute('d') !== initialGlyph
    && getComputedStyle(glyph.parentElement).opacity === '0'
    && query(lightsHost, '.c-traffic').dataset.dirty === '1', 'same lights respond to focus, dirty and maximize');
  throws(() => lights.update({ focused: true, maximized: 'false' }), 'invalid multi-prop traffic lights patch');
  assert(query(lightsHost, '.c-traffic').dataset.focused === '0', 'invalid patch is not partially applied');
  for (const button of lightButtons) button.click();
  await Promise.resolve();
  assert(events.splice(0).map(([value]) => value).join(' ') === 'close minimize zoom', 'three distinct window actions');
  unmount(lights);

  const gripperHost = target();
  const gripper = mount_widget(gripperHost, 'split-grip', { value: 200, min: 152, max: 284,
    label: 'Sidebar width', controls: 'preview' }, emit);
  const grip = query(gripperHost, '[role=separator][aria-controls=preview]');
  assert(grip && grip.tabIndex === 0 && grip.getAttribute('aria-valuenow') === '200', 'real accessible pixel grip');
  grip.focus();
  key(grip, 'ArrowRight');
  key(grip, 'ArrowLeft', { shiftKey: true });
  key(grip, 'Home');
  key(grip, 'ArrowLeft');
  await tick();
  assert(events.splice(0).map(([value]) => value).join(' ') === '208 184 152' && grip.getAttribute('aria-valuenow') === '152',
    'split grip step, Shift, Home and lower clamp');
  gripper.update({ value: 250, max: 240 });
  await tick();
  assert(document.activeElement === grip && grip.getAttribute('aria-valuenow') === '240'
    && grip.getAttribute('aria-valuemax') === '240' && !events.length, 'responsive split bounds update in place');
  unmount(gripper);

  const toolbarHost = target(), child = target(toolbarHost);
  const capsuleHost = target(child), buttonChild = capsuleHost.appendChild(document.createElement('button'));
  buttonChild.textContent = 'Live';
  const toolbar = mount_widget(toolbarHost, 'toolbar', {}, noop);
  const capsule = mount_widget(capsuleHost, 'capsule', { size: 'md' }, noop);
  const capsuleSurface = query(capsuleHost, '.c-capsule');
  const filter = document.getElementById('gd-glass-capsule-180');
  assert(query(toolbarHost, '.c-topbar').firstElementChild === child && capsuleSurface.contains(buttonChild)
    && capsuleSurface.offsetWidth === 180 && capsuleSurface.offsetHeight === 34
    && getComputedStyle(capsuleSurface).backdropFilter.includes('#gd-glass-capsule-180'),
    'actual projected Toolbar and fixed-size Capsule with a live SVG definition');
  const defs = filter.parentNode;
  filter.remove();
  assert(!getComputedStyle(capsuleSurface).backdropFilter.includes('url(')
    && getComputedStyle(capsuleSurface).backgroundImage.includes('radial-gradient'), 'missing SVG definition keeps real CSS material');
  defs.append(filter);
  unmount(capsule);
  assert(capsuleHost.firstElementChild === buttonChild, 'Capsule returns projected node on dispose');
  const auto = mount_widget(capsuleHost, 'capsule', {}, noop);
  assert(getComputedStyle(query(capsuleHost, '.c-capsule')).backgroundImage.includes('radial-gradient'), 'Auto Capsule always retains CSS material');
  unmount(auto);
  unmount(toolbar);
  assert(toolbarHost.firstElementChild === child, 'Toolbar returns projected child on dispose');
  const panelHost = target();
  const details = target(panelHost);
  const panel = mount_widget(panelHost, 'glass-panel', { size: '320x220' }, noop);
  const panelSurface = query(panelHost, '.c-glass-panel');
  assert(panelSurface.firstElementChild === details && panelSurface.offsetWidth === 320 && panelSurface.offsetHeight === 220
    && getComputedStyle(panelSurface).backdropFilter.includes('#gd-glass-panel-320x220'), 'fixed GlassPanel projects DOM and uses its own SVG map');
  unmount(panel);
  assert(!events.length, 'no unexpected emissions');
  assert(!errors.length, `no uncaught browser errors: ${errors.join('\n')}`);
}

try {
  await smoke();
  await fetch('/result', { method: 'POST', body: JSON.stringify({ checks }) });
} catch (error) {
  await fetch('/result', { method: 'POST', body: JSON.stringify({ error: `${error.stack}\n${errors.join('\n')}` }) });
}
