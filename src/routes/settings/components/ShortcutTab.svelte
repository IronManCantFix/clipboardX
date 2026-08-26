<script lang="ts">
  import { t } from '../../../lib/i18n';
  import type { SettingSetter, Settings } from '../../../lib/api';

  let { settings, setter }: { settings: Settings; setter: SettingSetter } = $props();

  let recording = $state(false);

  function startRecord() {
    recording = true;
    window.addEventListener('keydown', onKeyCapture);
  }

  function onKeyCapture(e: KeyboardEvent) {
    e.preventDefault();
    const mods: string[] = [];
    if (e.metaKey) mods.push('Command');
    if (e.ctrlKey) mods.push('Control');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    // 只接受可打印字符组合；F 键/方向键等（key 长度 > 1）忽略，避免录出无效组合
    if (mods.length === 0 || e.key.length !== 1) return;
    const combo = mods.join('+') + '+' + e.key.toUpperCase();
    recording = false;
    window.removeEventListener('keydown', onKeyCapture);
    setter('shortcut', combo);
  }
</script>

<section>
  <h2>{t('set_shortcut')}</h2>
  <div class="card">
    <div class="field row">
      <div>
        <span class="lbl">{t('shortcut_show')}</span>
        <p class="hint">{t('shortcut_record_hint')}</p>
      </div>
      <button class="keybtn" onclick={recording ? undefined : startRecord}>
        {recording ? t('recording') : settings.shortcut}
      </button>
    </div>
    <div class="divider"></div>
    <div class="field">
      <p class="hint">{t('popup_keys_hint')}</p>
    </div>
  </div>
</section>
