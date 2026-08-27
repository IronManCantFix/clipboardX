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
  /* macOS 26 液态玻璃的分段控件语言：胶囊容器 + 浮起的当前段 */
  .tabs {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    display: flex;
    gap: 2px;
    width: fit-content;
    margin: 0 11px 7px;
    padding: 2px;
    border-radius: 999px;
    background: rgba(var(--lg-tint), 0.24);
    border: 1px solid var(--lg-border);
    box-shadow: var(--lg-specular), var(--lg-occlusion);
  }
  button {
    background: transparent;
    border: none;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 4px 12px;
    border-radius: 999px;
    cursor: pointer;
    transition: background 0.18s, color 0.18s, box-shadow 0.18s;
  }
  button:hover {
    color: var(--text-primary);
  }
  button.active {
    color: var(--text-primary);
    background: rgba(var(--lg-tint), 0.6);
    box-shadow: var(--lg-specular), var(--lg-occlusion),
      0 1px 3px rgba(0, 0, 0, 0.1);
  }
</style>
