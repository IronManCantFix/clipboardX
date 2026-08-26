<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { api, type Entry, type StorageStats } from '../../../lib/api';
  import { t } from '../../../lib/i18n';
  import { fmtSize } from '../format';

  let {
    active,
    onStatus,
  }: {
    /** 首次切到本 tab 时才加载数据，与旧版懒加载行为一致 */
    active: boolean;
    onStatus: (msg: string) => void;
  } = $props();

  let stats = $state<StorageStats | null>(null);
  let loaded = $state(false);
  let entries = $state<Entry[]>([]);
  let historyOpen = $state(false);

  const MAX_DISPLAY = 50;

  const entryIconPaths: Record<string, string> = {
    text: '<path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/><path d="M9 13h6M9 17h4"/>',
    url: '<path d="M10 14l4-4"/><path d="M8 12l-2 2a4 4 0 006 6l2-2"/><path d="M16 12l2-2a4 4 0 00-6-6l-2 2"/>',
    image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M21 16l-5-5-9 9"/>',
    file: '<path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>',
    rtf: '<path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/><path d="M9 11h6M9 15h6M9 19h3"/>',
  };

  // 首次激活时加载存储统计（此后 active 变化不重复拉取）
  $effect(() => {
    if (active && !loaded) {
      loaded = true;
      void loadStats();
    }
  });

  async function loadStats() {
    stats = await api.getStorageStats();
  }

  async function toggleHistory() {
    if (historyOpen) {
      historyOpen = false;
      entries = [];
    } else {
      entries = await api.list();
      historyOpen = true;
    }
  }

  async function deleteOne(id: number) {
    await api.delete(id);
    entries = entries.filter((e) => e.id !== id);
    void loadStats();
  }

  async function clearHistory() {
    const ok = await confirm(t('confirm_clear_history'), { title: 'ClipboardX', kind: 'warning' });
    if (!ok) return;
    const n = await api.clearHistory();
    onStatus(t('st_cleared', { n }));
    entries = [];
    void loadStats();
  }

  async function clearImages() {
    const ok = await confirm(t('confirm_clear_images'), { title: 'ClipboardX', kind: 'warning' });
    if (!ok) return;
    await api.clearImages();
    onStatus(t('st_images_cleared'));
    void loadStats();
  }
</script>

<section class="storage-section">
  <h2>{t('set_storage')}</h2>
  {#if stats}
    <div class="stat-grid">
      <div class="stat"><b>{fmtSize(stats.db_size)}</b><span>{t('stat_db')}</span></div>
      <div class="stat"><b>{fmtSize(stats.images_size)}</b><span>{t('stat_images')}</span></div>
      <div class="stat"><b>{stats.entry_count}</b><span>{t('stat_entries')}</span></div>
    </div>

    <div class="card storage-card">
      <div class="storage-row">
        <div class="storage-info">
          <span class="lbl">{t('image_cache_label')}</span>
          <span class="hint">{t('files_count', { size: fmtSize(stats.images_size), n: stats.image_count })}</span>
        </div>
        <button class="danger" onclick={clearImages}>{t('btn_clear_cache')}</button>
      </div>
      <div class="divider"></div>
      <div class="storage-path">
        <span class="hint">{t('cache_location')}</span>
        <div class="path" title={stats.images_dir}>{stats.images_dir}</div>
      </div>
    </div>

    <div class="card storage-card">
      <div class="storage-row">
        <div class="storage-info">
          <span class="lbl">{t('clear_history_label')}</span>
          <span class="hint">{t('clear_history_hint')}</span>
        </div>
        <button class="danger" onclick={clearHistory}>{t('btn_clear_history')}</button>
      </div>
    </div>

    <div class="card storage-card">
      <div class="storage-row">
        <div class="storage-info">
          <span class="lbl">{t('records_label')}</span>
          <span class="hint">{historyOpen ? t('items_count', { n: entries.length }) : t('records_open_hint')}</span>
        </div>
        <button class="keybtn" onclick={toggleHistory}>{historyOpen ? t('btn_hide') : t('btn_view')}</button>
      </div>
      {#if historyOpen}
        <div class="divider"></div>
        <div class="entry-list">
          {#each entries.slice(0, MAX_DISPLAY) as e (e.id)}
            <div class="entry">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"
                   stroke-linecap="round" stroke-linejoin="round" class="e-ico e-{e.type}">
                {@html entryIconPaths[e.type]}
              </svg>
              <span class="e-txt" title={e.content}>
                {e.type === 'image' ? t('img_entry') : e.content}
              </span>
              {#if e.pinned}
                <svg class="e-pin" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M16 3l5 5-4 1-3 3 1 6-2 2-4-5-5 4-1-1 4-5-5-4 2-2 6 1 3-3 1-4z"/>
                </svg>
              {/if}
              <button class="e-del" aria-label={t('act_delete')} title={t('act_delete')}
                      onclick={() => deleteOne(e.id)}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                     stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
              </button>
            </div>
          {:else}
            <p class="hint" style="text-align:center;padding:12px 0">{t('empty_history')}</p>
          {/each}
        </div>
        {#if entries.length > MAX_DISPLAY}
          <p class="hint" style="text-align:center;padding-top:6px">{t('recent_only', { max: MAX_DISPLAY })}</p>
        {/if}
      {/if}
    </div>
  {:else}
    <p class="hint" style="padding:20px 0;text-align:center">{t('reading_stats')}</p>
  {/if}
</section>
