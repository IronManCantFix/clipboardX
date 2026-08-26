<script lang="ts">
  import { onMount } from 'svelte';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { listen } from '@tauri-apps/api/event';
  import { api } from '../../../lib/api';
  import { t } from '../../../lib/i18n';

  let { version }: { version: string } = $props();

  let updateMsg = $state('');
  let checking = $state(false);
  let unlistenProgress: (() => void) | undefined;

  onMount(() => {
    // 更新下载进度：install_update 通过 update:progress 事件回报
    void listen<{ downloaded: number; total: number | null }>('update:progress', (e) => {
      const { downloaded, total } = e.payload;
      updateMsg =
        total != null
          ? t('updating_pct', { pct: Math.min(100, Math.round((downloaded / total) * 100)) })
          : t('updating_bytes', { mb: (downloaded / 1024 / 1024).toFixed(1) });
    }).then((un) => (unlistenProgress = un));
    return () => unlistenProgress?.();
  });

  async function checkUpdate() {
    if (checking) return;
    checking = true;
    updateMsg = '';
    try {
      const info = await api.checkUpdate();
      if (!info.has_update) {
        updateMsg = t('up_to_date', { v: info.current_version });
        return;
      }
      const ok = await confirm(
        t('update_found', { latest: info.latest_version ?? '?', current: info.current_version }),
        { title: 'ClipboardX', kind: 'info' },
      );
      if (!ok) return;
      updateMsg = t('updating');
      await api.installUpdate();
      // 安装成功后应用会自动重启，此处不会继续执行
    } catch (e) {
      updateMsg = String(e).replace(/^Error:\s*/, '');
    } finally {
      checking = false;
    }
  }
</script>

<section>
  <h2>{t('set_about')}</h2>
  <div class="about-card">
    <img class="about-logo" src="/icon.png" alt="ClipboardX" />
    <b>ClipboardX</b>
    <span class="ver">{t('version_label', { v: version || '…' })}</span>
    <span class="author">{t('author_line')}</span>
    <p class="about-desc">{t('about_desc')}</p>
    <div class="about-links">
      <button class="gh-link" onclick={() => openUrl('https://github.com/IronManCantFix/clipboardX')}>GitHub ↗</button>
    </div>
  </div>
  <div class="card">
    <div class="field row">
      <div>
        <span class="lbl">{t('check_update')}</span>
        <p class="hint">{t('check_update_hint')}</p>
      </div>
      <button class="keybtn" onclick={checkUpdate} disabled={checking}>
        {checking ? t('checking') : t('check_update')}
      </button>
    </div>
    {#if updateMsg}
      <p class="hint update-msg">{updateMsg}</p>
    {/if}
  </div>
</section>
