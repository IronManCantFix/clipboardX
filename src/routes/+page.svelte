<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { api, onNewEntry, type Entry } from '../lib/api';
  import { filterEntries } from '../lib/search';
  import TabBar from '../components/TabBar.svelte';
  import EntryRow from '../components/EntryRow.svelte';

  let entries: Entry[] = $state([]);
  let query = $state('');
  let tab: 'all' | 'pinned' = $state('all');
  let selected = $state(0);
  let searchInput: HTMLInputElement | undefined = $state();

  let visibleEntries = $derived(filterEntries(entries, query));

  async function refresh() {
    entries = await api.list(tab === 'pinned' ? 'pinned' : undefined);
    selected = 0;
  }

  async function switchTab(t: 'all' | 'pinned') {
    tab = t;
    await refresh();
  }

  onMount(async () => {
    const win = getCurrentWindow();
    await refresh();
    onNewEntry(refresh);
    searchInput?.focus();
    // 失焦自动隐藏
    void win.onFocusChanged(({ payload: focused }) => {
      if (!focused) void win.hide();
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

  function openSettings() {
    void api.showSettings();
  }

  // 动态高度：条目 < 15 按实际数量，>= 15 按 15 行高度，多余滚动
  const ROW_H = 32;
  const FIXED_H = 110;
  const EMPTY_H = 100;
  const MAX_ROWS = 15;
  const WIN_W = 360;
  function applyHeight() {
    const n = Math.max(1, Math.min(visibleEntries.length, MAX_ROWS));
    const empty = visibleEntries.length === 0 ? EMPTY_H : 0;
    void getCurrentWindow().setSize(new LogicalSize(WIN_W, FIXED_H + n * ROW_H + empty));
  }
  $effect(() => {
    visibleEntries;
    applyHeight();
  });

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
    <input bind:this={searchInput} bind:value={query} placeholder="搜索剪贴板…" />
    <button class="gear" onclick={openSettings} title="设置">⚙︎</button>
  </div>
  <TabBar {tab} pinnedCount={entries.filter((e) => e.pinned).length} onswitch={switchTab} />
  <div class="list">
    {#if visibleEntries.length === 0}
      <div class="empty">{query ? '无匹配结果' : '暂无剪贴板记录'}</div>
    {:else}
      {#each visibleEntries as entry, i (entry.id)}
        <EntryRow
          {entry}
          selected={i === selected}
          {query}
          onselect={() => paste(entry)}
          onmouseenter={() => (selected = i)}
        />
      {/each}
    {/if}
  </div>
  <div class="foot">
    <span>↑↓ 选择 · ⏎ 粘贴</span>
    <span>⌘P 固定 · ⌫ 删除 · Esc 关闭</span>
  </div>
</div>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
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
