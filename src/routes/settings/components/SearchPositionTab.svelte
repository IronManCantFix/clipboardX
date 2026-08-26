<script lang="ts">
  import { t } from '../../../lib/i18n';
  import type { SettingSetter, Settings } from '../../../lib/api';

  let { settings, setter }: { settings: Settings; setter: SettingSetter } = $props();
</script>

<section>
  <h2>{t('set_search_position')}</h2>
  <div class="card">
    <div class="field row">
      <div>
        <span class="lbl">{t('search_enabled')}</span>
        <p class="hint">{t('search_enabled_hint')}</p>
      </div>
      <button class="toggle" class:on={settings.search_enabled} aria-label={t('search_enabled')}
              onclick={() => setter('search_enabled', !settings.search_enabled)}></button>
    </div>
    <div class="divider"></div>
    <div class="field">
      <div class="row-top"><span class="lbl">{t('search_mode')}</span></div>
      <div class="seg">
        <button class:on={settings.search_mode === 'fuzzy'} onclick={() => setter('search_mode', 'fuzzy')}>{t('search_fuzzy')}</button>
        <button class:on={settings.search_mode === 'exact'} onclick={() => setter('search_mode', 'exact')}>{t('search_exact')}</button>
      </div>
      <p class="hint">{t('fuzzy_hint')}</p>
    </div>
    <div class="divider"></div>
    <div class="field">
      <div class="row-top"><span class="lbl">{t('position_mode')}</span></div>
      <div class="seg">
        <button class:on={settings.position_mode === 'cursor'} onclick={() => setter('position_mode', 'cursor')}>{t('position_cursor')}</button>
        <button class:on={settings.position_mode === 'center'} onclick={() => setter('position_mode', 'center')}>{t('position_center')}</button>
      </div>
      <p class="hint">{t('cursor_hint')}</p>
    </div>
  </div>
</section>
