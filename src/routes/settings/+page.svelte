<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Settings } from '../../lib/api';

  let settings = $state<Settings>({ history_limit: 15, shortcut: 'Command+Shift+V' });
  let recording = $state(false);
  let status = $state('');

  onMount(async () => {
    settings = await api.getSettings();
  });

  async function save() {
    try {
      await api.updateSettings(settings);
      status = '已保存 ✓';
    } catch (e) {
      status = '保存失败：' + e;
    }
  }

  function onLimitInput(e: Event) {
    settings.history_limit = Number((e.target as HTMLInputElement).value);
    void save();
  }

  function startRecord() {
    recording = true;
    status = '请按下新的快捷键…';
    window.addEventListener('keydown', onKeyCapture);
  }

  function onKeyCapture(e: KeyboardEvent) {
    e.preventDefault();
    const mods: string[] = [];
    if (e.metaKey) mods.push('Command');
    if (e.ctrlKey) mods.push('Control');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    if (mods.length === 0 || e.key.length !== 1) return;
    settings.shortcut = mods.join('+') + '+' + e.key.toUpperCase();
    recording = false;
    window.removeEventListener('keydown', onKeyCapture);
    void save();
  }
</script>

<div class="settings">
  <h2>ClipboardX 设置</h2>

  <section>
    <h3>常规</h3>
    <label>
      历史记录上限（{settings.history_limit} 条，最大 100）
      <input
        type="range"
        min="1"
        max="100"
        value={settings.history_limit}
        oninput={onLimitInput}
      />
    </label>
    <p class="hint">超过上限自动删除最旧的未固定条目。</p>
  </section>

  <section>
    <h3>快捷键</h3>
    <button class="keybtn" onclick={recording ? undefined : startRecord}>
      {recording ? '按下组合键…' : settings.shortcut}
    </button>
  </section>

  <p class="status">{status}</p>
</div>

<style>
  .settings {
    padding: 20px 24px;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    color: var(--text-primary);
    background: var(--bg);
    height: 100vh;
    box-sizing: border-box;
  }
  :root {
    --bg: #f5f5f7;
    --text-primary: #1d1d1f;
    --text-secondary: #86868b;
    --border: #d1d1d6;
    --accent: #0a84ff;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #1d1d1f;
      --text-primary: #f5f5f7;
      --text-secondary: #98989d;
      --border: #424245;
      --accent: #0a84ff;
    }
  }
  h2 {
    font-size: 17px;
    margin: 0 0 16px;
  }
  section {
    margin-bottom: 22px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 18px;
  }
  h3 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    margin: 0 0 10px;
  }
  label {
    font-size: 13px;
  }
  input[type='range'] {
    width: 100%;
    margin-top: 8px;
    accent-color: var(--accent);
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-secondary);
    margin: 8px 0 0;
  }
  .keybtn {
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 7px 16px;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    cursor: pointer;
  }
  .status {
    color: var(--text-secondary);
    font-size: 12px;
    min-height: 16px;
  }
</style>
