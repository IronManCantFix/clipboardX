<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { api, onNewEntry, type Entry, type Settings } from '../lib/api';
  import { filterEntries } from '../lib/search';
  import { t } from '../lib/i18n';
  import TabBar from '../components/TabBar.svelte';
  import EntryRow from '../components/EntryRow.svelte';

  let entries: Entry[] = $state([]);
  let query = $state('');
  let tab: 'all' | 'pinned' = $state('all');
  let selected = $state(0);
  let searchInput: HTMLInputElement | undefined = $state();
  // 弹窗是否处于显示/焦点状态：隐藏期间收到新条目事件跳过刷新，
  // 避免常驻期反复全量查询 + 重渲染 + 对隐藏窗口 setSize/positionPopover
  let popupFocused = $state(false);
  let settings = $state<Settings>({
    history_limit: 15,
    shortcut: 'Command+Shift+V',
    opacity: 0.9,
    glass_strength: 0.7,
    width_mode: 'auto',
    height_mode: 'auto',
    window_width: 360,
    window_height: 480,
    search_enabled: true,
    search_mode: 'fuzzy',
    position_mode: 'cursor',
    image_preview_height: 60,
  });

  const ROW_H = 32;
  const EMPTY_H = 100;
  const MAX_LIST_H = 15 * ROW_H;
  const AUTO_W = 360;

  let unListenShown: (() => void) | undefined;

  // 主动隐藏弹窗（统一入口）。
  // blurWhileHidden：窗口隐藏期间 DOM blur 迟到的竞态——切 Tab 时渲染变更会
  // 瞬时抢走输入焦点（与原生失焦叠加），若此刻 hide 已执行，迟到的 blur 事件
  // 会把刚重新聚焦的窗口再藏一次。这里记录「我们主动 hide 过」，失焦监听器
  // 看到标记后忽略该次 blur（onMount 中注册，见下方）。
  let blurWhileHidden = false;
  function hideWindow() {
    blurWhileHidden = true;
    void getCurrentWindow().hide();
  }

  function applyFilter(list: Entry[]): Entry[] {
    return settings.search_enabled ? filterEntries(list, query, settings.search_mode) : list;
  }

  let pinnedCount = $derived(entries.filter((e) => e.pinned).length);

  let visibleEntries = $derived(applyFilter(entries));

  async function refresh() {
    const next = await api.list(tab === 'pinned' ? 'pinned' : undefined);
    // 新条目到达时尽量保持当前选中项（按 id 而非固定归零），避免浏览中被打断
    const prevId = visibleEntries[selected]?.id ?? null;
    entries = next;
    const filtered = applyFilter(next);
    if (prevId !== null) {
      const idx = filtered.findIndex((e) => e.id === prevId);
      selected = idx >= 0 ? idx : 0;
    } else {
      selected = Math.min(selected, Math.max(0, filtered.length - 1));
    }
  }

  async function switchTab(t: 'all' | 'pinned') {
    tab = t;
    await refresh();
  }

  function rowHeight(entry: Entry): number {
    return entry.type === 'image'
      ? Math.max(ROW_H, settings.image_preview_height + 12)
      : ROW_H;
  }

  // 固定区域（搜索框 / Tab / 底部提示）的实际渲染高度，随开关自动变化
  function fixedHeight(): number {
    let h = 12; // .list 上下 padding 4+6 + .pop 边框 1+1（与 app.css 的 .list 同步改）
    for (const sel of ['.search', '.tabs', '.foot']) {
      const el = document.querySelector<HTMLElement>(sel);
      if (el) h += el.offsetHeight;
    }
    return h;
  }

  // 列表内容高度：按实际渲染的行高累加，超过 15 行时用滚动条
  function listHeight(): number {
    if (visibleEntries.length === 0) return EMPTY_H;
    const rows = Array.from(document.querySelectorAll<HTMLElement>('.row'));
    const h =
      rows.length === visibleEntries.length
        ? rows.reduce((s, r) => s + r.offsetHeight, 0)
        : visibleEntries.reduce((s, e) => s + rowHeight(e), 0);
    return Math.min(h, MAX_LIST_H);
  }

  function applyHeight() {
    const win = getCurrentWindow();
    const width =
      settings.width_mode === 'custom' ? settings.window_width : AUTO_W;
    const height =
      settings.height_mode === 'custom'
        ? settings.window_height
        : fixedHeight() + listHeight();
    void win.setSize(new LogicalSize(width, height));
    // 传目标尺寸做锚点钳位：切 Tab 等内容变化不会让窗口跟着鼠标跑，
    // 且后端按最终尺寸钳位，贴屏幕底部时也不会伸出屏幕。
    void api.positionPopover({ width, height });
  }

  $effect(() => {
    visibleEntries;
    pinnedCount;
    settings.search_enabled;
    settings.width_mode;
    settings.height_mode;
    settings.window_width;
    settings.window_height;
    settings.image_preview_height;
    applyHeight();
  });

  // 键盘导航的滚动跟随：选中项超出可视区时滚入视野。
  // block:'nearest' 保证鼠标 hover 改变选中时不会产生多余滚动（元素已可见即原样）。
  $effect(() => {
    void selected;
    void visibleEntries.length;
    const raf = requestAnimationFrame(() => {
      document
        .querySelectorAll<HTMLElement>('.row')
        [selected]?.scrollIntoView({ block: 'nearest' });
    });
    return () => cancelAnimationFrame(raf);
  });

  // 固定项被清空时回到「全部」Tab，避免停留在空的固定列表
  $effect(() => {
    if (tab === 'pinned' && pinnedCount === 0) {
      tab = 'all';
      void refresh();
    }
  });

  // 应用透明度 + 高光反光/玻璃质感。
  // 1. 原生 Popover 模糊材质提供稳定无残影的毛玻璃底层。
  // 2. 通过 setWindowOpacity 动态调整系统级窗口透明度（透出桌面壁纸）。
  // 3. glass_strength 控制明显的高光反光：顶部光池、晶莹边框棱线、控件反光与内嵌高光。
  $effect(() => {
    void api.setWindowOpacity(settings.opacity);
    const gs = settings.glass_strength;
    const t = Math.max(0, Math.min(1, (gs - 0.2) / 0.8));
    const root = document.documentElement;
    const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

    // 1. 顶部光池强度
    root.style.setProperty('--glass-sheen-top', (0.04 + t * 0.46).toFixed(3));
    root.style.setProperty('--glass-sheen-mid', (0.01 + t * 0.12).toFixed(3));
    root.style.setProperty('--glass-sheen-dark-top', (0.03 + t * 0.35).toFixed(3));
    root.style.setProperty('--glass-sheen-dark-mid', (0.008 + t * 0.09).toFixed(3));

    // 2. 边框晶莹反光与棱线
    const borderA = isDark ? (0.10 + t * 0.35).toFixed(3) : (0.20 + t * 0.70).toFixed(3);
    const lgBorderA = isDark ? (0.12 + t * 0.32).toFixed(3) : (0.25 + t * 0.65).toFixed(3);
    root.style.setProperty('--pop-border', `rgba(255, 255, 255, ${borderA})`);
    root.style.setProperty('--lg-border', `rgba(255, 255, 255, ${lgBorderA})`);

    // 3. 内嵌高光（Specular Bevel）
    const specA = isDark ? (0.08 + t * 0.35).toFixed(3) : (0.20 + t * 0.75).toFixed(3);
    root.style.setProperty('--lg-specular', `inset 0 1px 1px rgba(255, 255, 255, ${specA})`);

    // 4. 搜索框与控件反光底色
    const fillA = isDark ? (0.06 + t * 0.20).toFixed(3) : (0.15 + t * 0.45).toFixed(3);
    const fillHoverA = isDark ? (0.14 + t * 0.25).toFixed(3) : (0.28 + t * 0.48).toFixed(3);
    root.style.setProperty('--glass-fill-idle', `rgba(255, 255, 255, ${fillA})`);
    root.style.setProperty('--glass-fill-hover', `rgba(255, 255, 255, ${fillHoverA})`);
  });

  onMount(async () => {
    const win = getCurrentWindow();
    try {
      settings = await api.getSettings();
    } catch {
      /* 使用默认值 */
    }
    await refresh();
    // 仅窗口可见时刷新；隐藏期间的变更在下次显示时统一刷新（见下方 onFocusChanged）
    onNewEntry(() => {
      if (popupFocused) void refresh();
    });
    searchInput?.focus();
    // 失焦自动隐藏（DOM blur + 窗口焦点事件双保险）。
    // blurWhileHidden 见 hideWindow 定义处：隐藏期间迟到的 blur 不再二次隐藏。
    const onBlur = () => {
      if (blurWhileHidden) {
        blurWhileHidden = false;
        return;
      }
      void win.hide();
    };
    window.addEventListener('blur', onBlur);
    void win.onFocusChanged(({ payload: focused }) => {
      popupFocused = focused;
      if (focused) {
        blurWhileHidden = false;
        // 每次显示时重新加载设置与列表，保证外观修改立即生效
        void api.getSettings().then((s) => (settings = s)).catch(() => {});
        void refresh();
      } else {
        hideWindow();
      }
    });
    // 唤起即重读设置：macOS accessory 窗口的 Focused 事件不可靠，
    // 仅靠 onFocusChanged 可能在设置改动后拿不到新值
    unListenShown = await listen('popover-shown', () => {
      void api.getSettings().then((s) => (settings = s)).catch(() => {});
      void refresh();
    });
  });

  onDestroy(() => unListenShown?.());

  function onKeydown(e: KeyboardEvent) {
    // 搜索框内按退格属于编辑搜索词，不应触发「删除条目」
    const inSearch = (e.target as HTMLElement | null)?.tagName === 'INPUT';
    if (inSearch && e.key === 'Backspace') return;
    if (e.key === 'Escape') {
      hideWindow();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selected = Math.min(selected + 1, visibleEntries.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (e.key === 'Enter' && visibleEntries[selected]) {
      e.preventDefault();
      paste(visibleEntries[selected]);
    } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'p') {
      e.preventDefault();
      const cur = visibleEntries[selected];
      if (cur) {
        void api.setPinned(cur.id, !cur.pinned).then(refresh);
      }
    } else if (e.key === 'Backspace' && visibleEntries[selected]) {
      void api.delete(visibleEntries[selected].id).then(refresh);
    } else if ((e.metaKey || e.ctrlKey) && e.key === '1') {
      void switchTab('all');
    } else if ((e.metaKey || e.ctrlKey) && e.key === '2') {
      void switchTab('pinned');
    }
  }

  async function paste(entry: Entry) {
    try {
      await api.paste(entry.id);
    } catch (err) {
      console.error('paste failed:', err);
    }
    hideWindow();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="pop">
  <div class="pop-bg"></div>
  {#if settings.search_enabled}
    <div class="search">
      <input bind:this={searchInput} bind:value={query} placeholder={t('ph_search')}
             aria-label={t('ph_search')} />
    </div>
  {/if}
  {#if pinnedCount > 0}
    <TabBar {tab} {pinnedCount} onswitch={switchTab} />
  {/if}
  <div class="list">
    {#if visibleEntries.length === 0}
      <div class="empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
             stroke-linecap="round" stroke-linejoin="round">
          <rect x="8" y="2" width="8" height="4" rx="1"/>
          <path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"/>
          <path d="M9 12h6M9 16h4"/>
        </svg>
        <p>{query ? t('empty_no_match') : t('empty_history')}</p>
        {#if !query}<span>{t('empty_hint')}</span>{/if}
      </div>
    {:else}
      {#each visibleEntries as entry, i (entry.id)}
        <EntryRow
          {entry}
          selected={i === selected}
          {query}
          thumbHeight={settings.image_preview_height}
          onselect={() => paste(entry)}
          onmouseenter={() => (selected = i)}
        />
      {/each}
    {/if}
  </div>
  <div class="foot">
    <span><kbd>↑↓</kbd> {t('act_select')} · <kbd>⏎</kbd> {t('act_paste')}</span>
    <span><kbd>⌘P</kbd> {t('act_pin')} · <kbd>⌫</kbd> {t('act_delete')} · <kbd>Esc</kbd> {t('act_close')}</span>
  </div>
</div>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
