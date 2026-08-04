<script lang="ts">
  import { highlight } from '../lib/search';
  import type { Entry } from '../lib/api';

  let {
    entry,
    selected,
    query,
    onselect,
    onmouseenter,
  }: {
    entry: Entry;
    selected: boolean;
    query: string;
    onselect: () => void;
    onmouseenter: () => void;
  } = $props();

  const typeLabel: Record<Entry['type'], string> = {
    text: 'T',
    rtf: 'R',
    image: '🖼',
    file: '🗂',
    url: '🔗',
  };
</script>

<div
  class="row"
  class:selected
  role="button"
  tabindex="-1"
  onclick={onselect}
  onmouseenter={onmouseenter}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onselect();
    }
  }}
>
  <span class="ico">{typeLabel[entry.type]}</span>
  <span class="txt">{@html highlight(entry.content, query)}</span>
  {#if entry.pinned}<span class="pin">📌</span>{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px;
    border-radius: 7px;
    cursor: pointer;
  }
  .row.selected {
    background: var(--selected-bg);
  }
  .ico {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: #fff;
    background: var(--icon-bg, #8e8e93);
  }
  .txt {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pin {
    flex-shrink: 0;
    font-size: 10px;
  }
  :global(mark) {
    background: var(--mark-bg);
    color: inherit;
    border-radius: 2px;
  }
</style>
