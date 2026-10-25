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

<!-- macOS 分段控件：全宽等宽两段，浮起的活动段；固定段带数量角标。
     本组件仅在 pinnedCount > 0 时由父组件渲染，两个段恒显示。 -->
<div class="tabs" role="tablist">
  <button role="tab" aria-selected={tab === 'all'} class:active={tab === 'all'}
          onclick={() => onswitch('all')}>{t('tab_all')}</button>
  <button role="tab" aria-selected={tab === 'pinned'} class:active={tab === 'pinned'}
          onclick={() => onswitch('pinned')}>
    {t('tab_pinned')}
    <span class="count">{pinnedCount}</span>
  </button>
</div>

<style>
  /* 分段控件容器：与搜索框同边距（11px），底部只留 2px——列表自身还有
     padding-top 4px，总空隙 6px，避免「Tab 与内容间距过大」的松散感 */
  .tabs {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    display: flex;
    gap: 2px;
    margin: 0 11px 2px;
    padding: 2px;
    border-radius: 8px;
    background: rgba(var(--lg-tint), 0.18);
    border: 1px solid var(--lg-border);
    box-shadow: var(--lg-specular), var(--lg-occlusion);
  }
  button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    background: transparent;
    border: none;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    padding: 3px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.18s, color 0.18s, box-shadow 0.18s;
  }
  button:hover:not(.active) {
    color: var(--text-primary);
    background: rgba(var(--lg-tint), 0.28);
  }
  button.active {
    color: var(--text-primary);
    font-weight: 600;
    background: rgba(var(--lg-tint), 0.7);
    box-shadow: var(--lg-specular), var(--lg-occlusion),
      0 1px 3px rgba(0, 0, 0, 0.12);
  }
  /* 固定数量角标：橙色呼应条目行的固定星标（.pin #ff9f0a） */
  .count {
    min-width: 14px;
    padding: 1px 5px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 600;
    line-height: 1.35;
    text-align: center;
    color: #ff9f0a;
    background: rgba(255, 159, 10, 0.14);
    box-shadow: inset 0 0 0 0.5px rgba(255, 159, 10, 0.28);
  }
  button.active .count {
    background: rgba(255, 159, 10, 0.22);
  }
</style>
