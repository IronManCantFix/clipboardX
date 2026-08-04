<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { api, onNewEntry, type Entry, type Settings } from '../lib/api';
  import { filterEntries } from '../lib/search';
  import TabBar from '../components/TabBar.svelte';
  import EntryRow from '../components/EntryRow.svelte';

  let entries: Entry[] = $state([]);
  let query = $state('');
  let tab: 'all' | 'pinned' = $state('all');
  let selected = $state(0);
  let searchInput: HTMLInputElement | undefined = $state();
  let settings = $state<Settings>({
    history_limit: 15,
    shortcut: 'Command+Shift+V',
    opacity: 0.9,
    window_mode: 'auto',
    window_width: 360,
    window_height: 480,
    search_enabled: true,
    search_mode: 'fuzzy',
    position_mode: 'cursor',
    image_preview_height: 60,
  });

  const ROW_H = 32;
  const FIXED_H = 110;
  const EMPTY_H = 100;
  const MAX_LIST_H = 15 * ROW_H;

  let visibleEntries = $derived(
    settings.search_enabled ? filterEntries(entries, query, settings.search_mode) : entries,
  );

  async function refresh() {
    entries = await api.list(tab === 'pinned' ? 'pinned' : undefined);
    selected = 0;
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

  function applyHeight() {
    const win = getCurrentWindow();
    if (settings.window_mode === 'custom') {
      void win.setSize(new LogicalSize(settings.window_width, settings.window_height));
    } else {
      const listH =
        visibleEntries.length === 0
          ? EMPTY_H
          : Math.min(
              visibleEntries.reduce((s, e) => s + rowHeight(e), 0),
              MAX_LIST_H,
            );
      void win.setSize(new LogicalSize(settings.window_width, FIXED_H + listH));
    }
    void api.positionPopover();
  }

  $effect(() => {
    visibleEntries;
    settings.window_mode;
    settings.window_width;
    settings.window_height;
    settings.image_preview_height;
    applyHeight();
  });

  // 应用透明度
  $effect(() => {
    document.documentElement.style.setProperty('--pop-alpha', String(settings.opacity));
  });

  function openSettings() {
    void api.showSettings();
  }

  onMount(async () => {
    const win = getCurrentWindow();
    try {
      settings = await api.getSettings();
    } catch {
      /* 使用默认值 */
    }
    await refresh();
    onNewEntry(refresh);
    searchInput?.focus();
    // 失焦自动隐藏（DOM blur + 窗口焦点事件双保险）
    const onBlur = () => void win.hide();
    window.addEventListener('blur', onBlur);
    void win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        // 每次显示时重新加载设置与列表，保证外观修改立即生效
        void api.getSettings().then((s) => (settings = s)).catch(() => {});
        void refresh();
      } else {
        void win.hide();
      }
    });
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selected = Math.min(selected + 1, visibleEntries.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (e.key === 'Enter' && visibleEntries[selected]) {
      e.preventDefault();
      paste(visibleEntries[selected]);
    } else if (e.key === 'Escape') {
      getCurrentWindow().hide();
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
      getCurrentWindow().hide();
    } catch (err) {
      console.error('paste failed:', err);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="pop">
  <div class="search">
    {#if settings.search_enabled}
      <input bind:this={searchInput} bind:value={query} placeholder="搜索剪贴板…" />
    {:else}
      <div class="no-search"></div>
    {/if}
    <button class="gear" onclick={openSettings} title="设置" aria-label="设置">⚙︎</button>
  </div>
  <TabBar {tab} pinnedCount={entries.filter((e) => e.pinned).length} onswitch={switchTab} />
  <div class="list">
    {#if visibleEntries.length === 0}
      <div class="empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
             stroke-linecap="round" stroke-linejoin="round">
          <rect x="8" y="2" width="8" height="4" rx="1"/>
          <path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"/>
          <path d="M9 12h6M9 16h4"/>
        </svg>
        <p>{query ? '无匹配结果' : '暂无剪贴板记录'}</p>
        {#if !query}<span>复制任何内容都会自动出现在这里</span>{/if}
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
    <span><kbd>↑↓</kbd> 选择 · <kbd>⏎</kbd> 粘贴</span>
    <span><kbd>⌘P</kbd> 固定 · <kbd>⌫</kbd> 删除 · <kbd>Esc</kbd> 关闭</span>
  </div>
</div>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .no-search {
    height: 28px;
  }
  .gear {
    background: none;
    border: none;
    font-size: 15px;
    cursor: pointer;
    color: var(--text-secondary);
    padding: 3px 4px;
    border-radius: 5px;
    flex-shrink: 0;
  }
  .gear:hover {
    color: var(--text-primary);
    background: rgba(120, 120, 128, 0.15);
  }
</style>
