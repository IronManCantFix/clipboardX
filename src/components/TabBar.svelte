<script lang="ts">
  import { t } from '../lib/i18n';

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
  <button class:active={tab === 'all'} onclick={() => onswitch('all')}>{t('tab_all')}</button>
  {#if pinnedCount > 0}
    <button class:active={tab === 'pinned'} onclick={() => onswitch('pinned')}>{t('tab_pinned')}</button>
  {/if}
</div>

<style>
  .tabs {
    flex-shrink: 0;
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
