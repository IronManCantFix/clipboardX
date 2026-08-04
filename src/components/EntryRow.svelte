<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { highlight } from '../lib/search';
  import type { Entry } from '../lib/api';

  let {
    entry,
    selected,
    query,
    thumbHeight,
    onselect,
    onmouseenter,
  }: {
    entry: Entry;
    selected: boolean;
    query: string;
    thumbHeight: number;
    onselect: () => void;
    onmouseenter: () => void;
  } = $props();

  const paths: Record<Entry['type'], string> = {
    text: '<path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/><path d="M9 13h6M9 17h4"/>',
    rtf: '<path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/><path d="M9 11h6M9 15h6M9 19h3"/>',
    url: '<path d="M10 14l4-4"/><path d="M8 12l-2 2a4 4 0 006 6l2-2"/><path d="M16 12l2-2a4 4 0 00-6-6l-2 2"/>',
    image:
      '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M21 16l-5-5-9 9"/>',
    file: '<path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>',
  };
</script>

<div
  class="row"
  class:selected
  class:is-image={entry.type === 'image'}
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
  {#if entry.type === 'image' && entry.image_path}
    <img
      class="thumb"
      src={convertFileSrc(entry.image_path)}
      alt="图片"
      style="height:{thumbHeight}px"
    />
    <span class="txt">图片</span>
  {:else}
    <svg class="ico ico-{entry.type}" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
      {@html paths[entry.type]}
    </svg>
    <span class="txt">{@html highlight(entry.content, query)}</span>
  {/if}
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
    flex-shrink: 0;
  }
  .ico-text { color: #0a84ff; }
  .ico-rtf { color: #64d2ff; }
  .ico-url { color: #34c759; }
  .ico-image { color: #bf5af2; }
  .ico-file { color: #ff9f0a; }
  .thumb {
    border-radius: 6px;
    flex-shrink: 0;
    object-fit: cover;
    width: auto;
    max-width: 90px;
    border: 1px solid var(--border);
  }
  .txt {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
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
