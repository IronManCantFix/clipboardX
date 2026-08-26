<script lang="ts">
  import { t } from '../../../lib/i18n';
  import type { SettingSetter, Settings } from '../../../lib/api';
  import { pct } from '../format';

  let { settings, setter }: { settings: Settings; setter: SettingSetter } = $props();
</script>

<section>
  <h2>{t('set_appearance')}</h2>
  <div class="card">
    <div class="field">
      <div class="row-top">
        <span class="lbl">{t('opacity')}</span>
        <span class="val">{Math.round(settings.opacity * 100)}%</span>
      </div>
      <input type="range" min="0.3" max="1" step="0.05" value={settings.opacity}
             style="--p:{pct(settings.opacity, 0.3, 1)}%"
             aria-label={t('opacity')}
             oninput={(e) => setter('opacity', Number((e.target as HTMLInputElement).value))} />
    </div>
    <div class="divider"></div>
    <div class="field">
      <div class="row-top"><span class="lbl">{t('win_width')}</span></div>
      <div class="seg">
        <button class:on={settings.width_mode === 'auto'} onclick={() => setter('width_mode', 'auto')}>{t('mode_auto')}</button>
        <button class:on={settings.width_mode === 'custom'} onclick={() => setter('width_mode', 'custom')}>{t('mode_custom')}</button>
      </div>
      {#if settings.width_mode === 'custom'}
        <div class="sub">
          <input type="range" min="220" max="800" step="10" value={settings.window_width}
                 style="--p:{pct(settings.window_width, 220, 800)}%"
                 aria-label={t('win_width')}
                 oninput={(e) => setter('window_width', Number((e.target as HTMLInputElement).value))} />
        </div>
      {/if}
      <p class="hint">{settings.width_mode === 'auto'
        ? t('width_auto_hint')
        : t('width_custom_hint', { w: settings.window_width })}</p>
    </div>
    <div class="divider"></div>
    <div class="field">
      <div class="row-top"><span class="lbl">{t('win_height')}</span></div>
      <div class="seg">
        <button class:on={settings.height_mode === 'auto'} onclick={() => setter('height_mode', 'auto')}>{t('mode_auto')}</button>
        <button class:on={settings.height_mode === 'custom'} onclick={() => setter('height_mode', 'custom')}>{t('mode_custom')}</button>
      </div>
      {#if settings.height_mode === 'custom'}
        <div class="sub">
          <input type="range" min="200" max="1200" step="10" value={settings.window_height}
                 style="--p:{pct(settings.window_height, 200, 1200)}%"
                 aria-label={t('win_height')}
                 oninput={(e) => setter('window_height', Number((e.target as HTMLInputElement).value))} />
        </div>
      {/if}
      <p class="hint">{settings.height_mode === 'auto'
        ? t('height_auto_hint')
        : t('height_custom_hint', { h: settings.window_height })}</p>
    </div>
    <div class="divider"></div>
    <div class="field">
      <div class="row-top">
        <span class="lbl">{t('thumb_height')}</span>
        <span class="val">{settings.image_preview_height} px</span>
      </div>
      <input type="range" min="24" max="200" step="4" value={settings.image_preview_height}
             style="--p:{pct(settings.image_preview_height, 24, 200)}%"
             aria-label={t('thumb_height')}
             oninput={(e) => setter('image_preview_height', Number((e.target as HTMLInputElement).value))} />
    </div>
  </div>
</section>
