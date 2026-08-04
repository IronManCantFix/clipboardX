<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
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
    await refresh();
    onNewEntry(refresh);
    searchInput?.focus();
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
    <input bind:this={searchInput} bind:value={query} placeholder="搜索剪贴板…" />
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
