<script lang="ts">
  import { t } from '../../../lib/i18n';
  import type { SettingSetter, Settings } from '../../../lib/api';
  import { pct } from '../format';

  let {
    settings,
    setter,
    autostart,
    onToggleAutostart,
  }: {
    settings: Settings;
    setter: SettingSetter;
    autostart: boolean;
    onToggleAutostart: () => void;
  } = $props();
</script>

<section>
  <h2>{t('set_general')}</h2>
  <div class="card">
    <div class="field row">
      <div>
        <span class="lbl">{t('autostart')}</span>
        <p class="hint">{t('autostart_hint')}</p>
      </div>
      <button class="toggle" class:on={autostart} aria-label={t('autostart')}
              onclick={onToggleAutostart}></button>
    </div>
    <div class="divider"></div>
    <div class="field">
      <div class="row-top">
        <span class="lbl">{t('history_limit')}</span>
        <span class="val">{t('items_count', { n: settings.history_limit })}</span>
      </div>
      <input type="range" min="1" max="100" value={settings.history_limit}
             style="--p:{pct(settings.history_limit, 1, 100)}%"
             aria-label={t('history_limit')}
             oninput={(e) => setter('history_limit', Number((e.target as HTMLInputElement).value))} />
      <p class="hint">{t('history_limit_hint')}</p>
    </div>
    <div class="divider"></div>
    <div class="field">
      <span class="lbl">{t('ax_title')}</span>
      <div class="notice">
        {t('ax_notice_1')}<br />
        {t('ax_notice_2_prefix')}<b>{t('ax_notice_path')}</b>{t('ax_notice_2_suffix')}
      </div>
    </div>
  </div>
</section>
