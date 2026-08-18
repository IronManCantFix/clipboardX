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
  <span class="badge type-{entry.type}">
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"
      stroke-linecap="round" stroke-linejoin="round">
      {#if entry.type === 'text'}
        <path d="M3 4h10M3 7.5h10M3 11h6" />
      {:else if entry.type === 'rtf'}
        <path d="M4 2.5h5L12 5.5v8H4z" />
        <path d="M9 2.5V5.5h3" />
        <path d="M6 8h4M6 10.5h3" />
      {:else if entry.type === 'url'}
        <path d="M9.5 6.5L11 5a2.4 2.4 0 013.4 3.4l-1.5 1.5" />
        <path d="M6.5 9.5L5 11a2.4 2.4 0 01-3.4-3.4l1.5-1.5" />
        <path d="M6 10l4-4" />
      {:else if entry.type === 'image'}
        <rect x="2.5" y="3" width="11" height="10" rx="1.5" />
        <circle cx="6" cy="6.5" r="1.2" />
        <path d="M13.5 10.5l-3-3-5 5" />
      {:else if entry.type === 'file'}
        <path d="M4 2.5h5l3 3v8H4z" />
        <path d="M9 2.5V5.5h3" />
      {/if}
    </svg>
  </span>
  {#if entry.type === 'image' && (entry.image_path || entry.thumb_path)}
    <img
      class="thumb"
      src={convertFileSrc(entry.thumb_path ?? entry.image_path ?? '')}
      alt=""
      loading="lazy"
      decoding="async"
      style="height:{thumbHeight}px"
    />
  {:else}
    <span class="txt">{@html highlight(entry.content, query)}</span>
  {/if}
  {#if entry.pinned}
    <svg class="pin" viewBox="0 0 16 16" fill="currentColor">
      <path d="M8 1.5l2 4.2 4.5.5-3.3 3 .9 4.5L8 11.6 3.9 13.7l.9-4.5-3.3-3 4.5-.5z" />
    </svg>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 7px;
    cursor: pointer;
    transition: background 0.12s;
  }
  .row:hover {
    background: var(--row-hover-bg, rgba(120, 120, 128, 0.12));
  }
  .row.selected {
    background: var(--selected-bg);
  }
  .badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    flex-shrink: 0;
  }
  .badge svg {
    width: 14px;
    height: 14px;
  }
  .type-text  { color: #86868b; background: rgba(134, 134, 139, 0.12); }
  .type-rtf   { color: #64d2ff; background: rgba(100, 210, 255, 0.13); }
  .type-url   { color: #34c759; background: rgba(52, 199, 89, 0.13); }
  .type-image { color: #bf5af2; background: rgba(191, 90, 242, 0.13); }
  .type-file  { color: #ff9f0a; background: rgba(255, 159, 10, 0.13); }
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
    font-size: 13px;
  }
  .pin {
    flex-shrink: 0;
    width: 11px;
    height: 11px;
    color: #ff9f0a;
    opacity: 0.85;
  }
  :global(mark) {
    background: var(--mark-bg);
    color: inherit;
    border-radius: 2px;
  }
</style>
