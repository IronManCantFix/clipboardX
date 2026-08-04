<script lang="ts">
  let {
    tab,
    pinnedCount,
    onswitch,
  }: {
    tab: 'all' | 'pinned';
    pinnedCount: number;
    onswitch: (t: 'all' | 'pinned') => void;
  } = $props();
</script>

<div class="tabs">
  <button class:active={tab === 'all'} onclick={() => onswitch('all')}>剪贴板</button>
  {#if pinnedCount > 0}
    <button class:active={tab === 'pinned'} onclick={() => onswitch('pinned')}>固定</button>
  {/if}
</div>

<style>
  .tabs {
    display: flex;
    gap: 14px;
    padding: 2px 10px 6px;
  }
  button {
    background: none;
    border: none;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 4px 2px 7px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }
  button.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }
</style>
