<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Settings, type StorageStats } from '../../lib/api';

  let settings = $state<Settings>({
    history_limit: 15,
    shortcut: 'Command+Shift+V',
    opacity: 0.9,
    window_mode: 'auto',
    window_width: 360,
    window_height: 480,
    search_enabled: true,
    search_mode: 'fuzzy',
    position_mode: 'cursor',
    image_preview_height: 60,
  });
  let stats = $state<StorageStats | null>(null);
  let recording = $state(false);
  let status = $state('');
  let savedAt = $state('');

  onMount(async () => {
    settings = await api.getSettings();
    void loadStats();
  });

  async function loadStats() {
    stats = await api.getStorageStats();
  }

  async function save() {
    try {
      await api.updateSettings(settings);
      status = '已保存 ✓';
      savedAt = new Date().toLocaleTimeString();
    } catch (e) {
      status = '保存失败：' + e;
    }
  }

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    settings[key] = value;
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

  async function clearImages() {
    if (!confirm('确定清除全部图片缓存吗？图片历史条目也会被删除。')) return;
    await api.clearImages();
    status = '图片缓存已清除';
    void loadStats();
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / 1024 / 1024).toFixed(2) + ' MB';
  }
</script>

<div class="settings">
  <header>
    <h2>ClipboardX 设置</h2>
    <span class="saved">{status}</span>
  </header>

  <section>
    <h3>常规</h3>
    <div class="field">
      <div class="lbl"><b>历史记录上限</b><span>超过上限自动删除最旧的未固定条目</span></div>
      <input
        type="range" min="1" max="100"
        value={settings.history_limit}
        oninput={(e) => set('history_limit', Number((e.target as HTMLInputElement).value))}
      />
      <div class="value">{settings.history_limit} 条</div>
    </div>
  </section>

  <section>
    <h3>外观</h3>
    <div class="field">
      <div class="lbl"><b>弹窗透明度</b></div>
      <input
        type="range" min="0.3" max="1" step="0.05"
        value={settings.opacity}
        oninput={(e) => set('opacity', Number((e.target as HTMLInputElement).value))}
      />
      <div class="value">{Math.round(settings.opacity * 100)}%</div>
    </div>

    <div class="field">
      <div class="lbl"><b>窗口尺寸</b></div>
      <div class="seg">
        <button class:on={settings.window_mode === 'auto'} onclick={() => set('window_mode', 'auto')}>自动</button>
        <button class:on={settings.window_mode === 'custom'} onclick={() => set('window_mode', 'custom')}>自定义</button>
      </div>
    </div>

    {#if settings.window_mode === 'custom'}
      <div class="field">
        <div class="lbl"><b>宽度</b></div>
        <input type="range" min="220" max="800" step="10" value={settings.window_width}
               oninput={(e) => set('window_width', Number((e.target as HTMLInputElement).value))} />
        <div class="value">{settings.window_width} px</div>
      </div>
      <div class="field">
        <div class="lbl"><b>高度</b></div>
        <input type="range" min="200" max="1200" step="10" value={settings.window_height}
               oninput={(e) => set('window_height', Number((e.target as HTMLInputElement).value))} />
        <div class="value">{settings.window_height} px</div>
      </div>
    {/if}

    <div class="field">
      <div class="lbl"><b>图片预览高度</b><span>弹窗内图片缩略图的大小</span></div>
      <input type="range" min="24" max="200" step="4" value={settings.image_preview_height}
             oninput={(e) => set('image_preview_height', Number((e.target as HTMLInputElement).value))} />
      <div class="value">{settings.image_preview_height} px</div>
    </div>
  </section>

  <section>
    <h3>搜索</h3>
    <div class="field row">
      <div class="lbl"><b>启用搜索框</b></div>
      <button class="toggle" class:on={settings.search_enabled} aria-label="启用搜索框"
              onclick={() => set('search_enabled', !settings.search_enabled)}></button>
    </div>
    <div class="field">
      <div class="lbl"><b>搜索模式</b></div>
      <div class="seg">
        <button class:on={settings.search_mode === 'fuzzy'} onclick={() => set('search_mode', 'fuzzy')}>模糊</button>
        <button class:on={settings.search_mode === 'exact'} onclick={() => set('search_mode', 'exact')}>精准</button>
      </div>
    </div>
  </section>

  <section>
    <h3>位置</h3>
    <div class="field">
      <div class="lbl"><b>弹窗出现位置</b><span>跟随鼠标会避开屏幕边缘</span></div>
      <div class="seg">
        <button class:on={settings.position_mode === 'cursor'} onclick={() => set('position_mode', 'cursor')}>跟随鼠标</button>
        <button class:on={settings.position_mode === 'center'} onclick={() => set('position_mode', 'center')}>屏幕居中</button>
      </div>
    </div>
  </section>

  <section>
    <h3>快捷键</h3>
    <div class="field row">
      <div class="lbl"><b>唤起剪贴板</b><span>点击后按下新的组合键</span></div>
      <button class="keybtn" onclick={recording ? undefined : startRecord}>
        {recording ? '按下组合键…' : settings.shortcut}
      </button>
    </div>
  </section>

  <section>
    <h3>存储</h3>
    {#if stats}
      <div class="stat-grid">
        <div class="stat"><b>{fmtSize(stats.db_size)}</b><span>数据库</span></div>
        <div class="stat"><b>{fmtSize(stats.images_size)}</b><span>图片缓存（{stats.image_count} 张）</span></div>
        <div class="stat"><b>{stats.entry_count}</b><span>历史条目</span></div>
      </div>
      <div class="path" title={stats.images_dir}>{stats.images_dir}</div>
      <button class="danger" onclick={clearImages}>清除图片缓存</button>
    {:else}
      <p class="hint">读取存储信息中…</p>
    {/if}
  </section>
</div>

<style>
  :root {
    --bg: #f5f5f7;
    --card: #ffffff;
    --text: #1d1d1f;
    --muted: #86868b;
    --border: #e3e3e8;
    --accent: #0a84ff;
    --danger: #ff3b30;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #1e1e20;
      --card: #2c2c2e;
      --text: #f5f5f7;
      --muted: #98989d;
      --border: #3a3a3c;
      --accent: #0a84ff;
      --danger: #ff453a;
    }
  }
  .settings {
    padding: 18px 22px 30px;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    color: var(--text);
    background: var(--bg);
    min-height: 100vh;
    box-sizing: border-box;
    font-size: 13px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }
  h2 { font-size: 17px; margin: 0; }
  .saved { color: var(--muted); font-size: 11.5px; }
  section {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px 16px;
    margin-bottom: 12px;
  }
  h3 {
    font-size: 11.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    margin: 0 0 12px;
  }
  .field { margin-bottom: 14px; }
  .field:last-child { margin-bottom: 0; }
  .field.row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .lbl b { display: block; font-weight: 600; }
  .lbl span { font-size: 11px; color: var(--muted); }
  input[type='range'] {
    width: 100%;
    margin-top: 8px;
    accent-color: var(--accent);
  }
  .value { font-size: 12px; color: var(--muted); text-align: right; margin-top: 2px; }
  .seg { display: flex; gap: 6px; margin-top: 6px; }
  .seg button {
    flex: 1;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    border-radius: 7px;
    padding: 6px 0;
    font-size: 12.5px;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .toggle {
    width: 40px;
    height: 24px;
    border-radius: 12px;
    border: none;
    background: var(--border);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
  }
  .toggle::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: left 0.15s;
  }
  .toggle.on { background: var(--accent); }
  .toggle.on::after { left: 18px; }
  .keybtn {
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 14px;
    background: var(--accent);
    color: #fff;
    font-size: 12.5px;
    cursor: pointer;
  }
  .stat-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin-bottom: 10px; }
  .stat {
    background: var(--bg);
    border-radius: 8px;
    padding: 10px 8px;
    text-align: center;
  }
  .stat b { display: block; font-size: 14px; }
  .stat span { font-size: 10.5px; color: var(--muted); }
  .path {
    font-size: 10px;
    color: var(--muted);
    background: var(--bg);
    border-radius: 6px;
    padding: 6px 8px;
    margin-bottom: 10px;
    word-break: break-all;
    font-family: ui-monospace, monospace;
  }
  .danger {
    border: 1px solid var(--danger);
    color: var(--danger);
    background: transparent;
    border-radius: 7px;
    padding: 6px 14px;
    font-size: 12.5px;
    cursor: pointer;
  }
  .hint { color: var(--muted); font-size: 12px; }
</style>
