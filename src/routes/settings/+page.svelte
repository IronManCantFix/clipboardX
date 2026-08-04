<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
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
  let autostart = $state(false);
  let version = $state('');
  let updateMsg = $state('');
  let checking = $state(false);
  let recording = $state(false);
  let status = $state('');

  onMount(async () => {
    settings = await api.getSettings();
    void loadStats();
    void api.getAutostart().then((v) => (autostart = v)).catch(() => {});
    void getVersion().then((v) => (version = v)).catch(() => {});
  });

  async function checkUpdate() {
    checking = true;
    updateMsg = '';
    try {
      const info = await api.checkForUpdates();
      updateMsg = info.has_update
        ? `发现新版本 ${info.latest_version}（当前 ${info.current_version}）`
        : `已是最新版本（${info.current_version}）`;
    } catch (e) {
      updateMsg = String(e).replace(/^Error:\s*/, '');
    } finally {
      checking = false;
    }
  }

  async function loadStats() {
    stats = await api.getStorageStats();
  }

  async function save() {
    try {
      await api.updateSettings(settings);
      status = '已保存';
    } catch (e) {
      status = '保存失败';
      console.error(e);
    }
  }

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    settings[key] = value;
    void save();
  }

  function startRecord() {
    recording = true;
    status = '按下新的组合键…';
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

  function pct(v: number, min: number, max: number): string {
    return String(((v - min) / (max - min)) * 100);
  }
</script>

<div class="settings">
  <header>
    <h1>ClipboardX</h1>
    <p class="subtitle">剪贴板管理器 · 设置{status ? ` · ${status}` : ''}</p>
  </header>

  <section>
    <h3><span class="ic">⚙️</span>常规</h3>
    <div class="field">
      <div class="row-top">
        <span class="lbl">历史记录上限</span>
        <span class="val">{settings.history_limit} 条</span>
      </div>
      <input
        type="range" min="1" max="100"
        value={settings.history_limit}
        style="--p:{pct(settings.history_limit, 1, 100)}%"
        oninput={(e) => set('history_limit', Number((e.target as HTMLInputElement).value))}
      />
      <p class="hint">超过上限自动删除最旧的未固定条目</p>
    </div>
  </section>

  <section>
    <h3><span class="ic">🎨</span>外观</h3>
    <div class="field">
      <div class="row-top">
        <span class="lbl">弹窗透明度</span>
        <span class="val">{Math.round(settings.opacity * 100)}%</span>
      </div>
      <input
        type="range" min="0.3" max="1" step="0.05"
        value={settings.opacity}
        style="--p:{pct(settings.opacity, 0.3, 1)}%"
        oninput={(e) => set('opacity', Number((e.target as HTMLInputElement).value))}
      />
    </div>

    <div class="field">
      <div class="row-top">
        <span class="lbl">窗口尺寸</span>
      </div>
      <div class="seg">
        <button class:on={settings.window_mode === 'auto'} onclick={() => set('window_mode', 'auto')}>自动</button>
        <button class:on={settings.window_mode === 'custom'} onclick={() => set('window_mode', 'custom')}>自定义</button>
      </div>
    </div>

    {#if settings.window_mode === 'custom'}
      <div class="field sub">
        <div class="row-top">
          <span class="lbl">宽度</span>
          <span class="val">{settings.window_width} px</span>
        </div>
        <input type="range" min="220" max="800" step="10" value={settings.window_width}
               style="--p:{pct(settings.window_width, 220, 800)}%"
               oninput={(e) => set('window_width', Number((e.target as HTMLInputElement).value))} />
      </div>
      <div class="field sub">
        <div class="row-top">
          <span class="lbl">高度</span>
          <span class="val">{settings.window_height} px</span>
        </div>
        <input type="range" min="200" max="1200" step="10" value={settings.window_height}
               style="--p:{pct(settings.window_height, 200, 1200)}%"
               oninput={(e) => set('window_height', Number((e.target as HTMLInputElement).value))} />
      </div>
    {/if}

    <div class="field">
      <div class="row-top">
        <span class="lbl">图片预览高度</span>
        <span class="val">{settings.image_preview_height} px</span>
      </div>
      <input
        type="range" min="24" max="200" step="4" value={settings.image_preview_height}
        style="--p:{pct(settings.image_preview_height, 24, 200)}%"
        oninput={(e) => set('image_preview_height', Number((e.target as HTMLInputElement).value))}
      />
    </div>
  </section>

  <section>
    <h3><span class="ic">🔍</span>搜索</h3>
    <div class="field row">
      <div>
        <span class="lbl">启用搜索框</span>
        <p class="hint">在弹窗顶部显示搜索输入</p>
      </div>
      <button class="toggle" class:on={settings.search_enabled} aria-label="启用搜索框"
              onclick={() => set('search_enabled', !settings.search_enabled)}></button>
    </div>
    <div class="field">
      <div class="row-top"><span class="lbl">搜索模式</span></div>
      <div class="seg">
        <button class:on={settings.search_mode === 'fuzzy'} onclick={() => set('search_mode', 'fuzzy')}>模糊匹配</button>
        <button class:on={settings.search_mode === 'exact'} onclick={() => set('search_mode', 'exact')}>精准匹配</button>
      </div>
      <p class="hint">模糊：按字符顺序匹配，如「gpo」可匹配「git push origin」</p>
    </div>
  </section>

  <section>
    <h3><span class="ic">📍</span>位置</h3>
    <div class="field">
      <div class="row-top"><span class="lbl">弹窗出现位置</span></div>
      <div class="seg">
        <button class:on={settings.position_mode === 'cursor'} onclick={() => set('position_mode', 'cursor')}>跟随鼠标</button>
        <button class:on={settings.position_mode === 'center'} onclick={() => set('position_mode', 'center')}>屏幕居中</button>
      </div>
      <p class="hint">跟随鼠标时会自动避开屏幕边缘</p>
    </div>
  </section>

  <section>
    <h3><span class="ic">⌨️</span>快捷键</h3>
    <div class="field row">
      <div>
        <span class="lbl">唤起剪贴板</span>
        <p class="hint">点击右侧按钮后按下新的组合键</p>
      </div>
      <button class="keybtn" onclick={recording ? undefined : startRecord}>
        {recording ? '按下组合键…' : settings.shortcut}
      </button>
    </div>
  </section>

  <section>
    <h3><span class="ic">🚀</span>启动</h3>
    <div class="field row">
      <div>
        <span class="lbl">开机自动启动</span>
        <p class="hint">登录系统后自动在后台运行</p>
      </div>
      <button class="toggle" class:on={autostart} aria-label="开机自动启动"
              onclick={() => {
                autostart = !autostart;
                void api.setAutostart(autostart).catch(() => {
                  autostart = !autostart;
                  status = '设置失败';
                });
              }}></button>
    </div>
  </section>

  <section>
    <h3><span class="ic">🔄</span>更新</h3>
    <div class="field row">
      <div>
        <span class="lbl">当前版本</span>
        <p class="hint">{version ? 'ClipboardX ' + version : '读取中…'}</p>
      </div>
      <button class="keybtn" onclick={checkUpdate} disabled={checking}>
        {checking ? '检查中…' : '检查更新'}
      </button>
    </div>
    {#if updateMsg}
      <p class="hint update-msg">{updateMsg}</p>
    {/if}
  </section>

  <section>
    <h3><span class="ic">💾</span>存储</h3>
    {#if stats}
      <div class="stat-grid">
        <div class="stat"><b>{fmtSize(stats.db_size)}</b><span>数据库</span></div>
        <div class="stat"><b>{fmtSize(stats.images_size)}</b><span>图片缓存</span></div>
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
  :global(html),
  :global(body) {
    overflow: auto;
    background: var(--bg);
  }

  :root {
    --bg: #f2f2f7;
    --card: #ffffff;
    --text: #1d1d1f;
    --muted: #86868b;
    --faint: #aeaeb2;
    --border: rgba(0, 0, 0, 0.08);
    --accent: #0a84ff;
    --danger: #ff3b30;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #1c1c1e;
      --card: #2c2c2e;
      --text: #f5f5f7;
      --muted: #98989d;
      --faint: #636366;
      --border: rgba(255, 255, 255, 0.09);
      --accent: #0a84ff;
      --danger: #ff453a;
    }
  }

  .settings {
    padding: 26px 24px 48px;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif;
    color: var(--text);
    box-sizing: border-box;
    font-size: 13px;
    max-width: 460px;
    margin: 0 auto;
  }

  header {
    margin-bottom: 20px;
    padding: 0 2px;
  }
  h1 {
    font-size: 24px;
    font-weight: 700;
    margin: 0 0 3px;
    letter-spacing: -0.02em;
  }
  .subtitle {
    color: var(--muted);
    font-size: 12.5px;
    margin: 0;
  }

  section {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 16px 18px;
    margin-bottom: 14px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  }

  h3 {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    margin: 0 0 14px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .ic {
    font-size: 13px;
  }

  .field {
    margin-bottom: 16px;
  }
  .field:last-child {
    margin-bottom: 0;
  }
  .field.sub {
    padding-left: 4px;
    border-left: 2px solid var(--border);
    margin-left: 2px;
    padding-bottom: 2px;
  }
  .field.row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .row-top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  .lbl {
    font-weight: 500;
    font-size: 13px;
  }
  .val {
    color: var(--muted);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--faint);
    font-size: 11.5px;
    margin: 4px 0 0;
    line-height: 1.4;
  }

  /* macOS 风格滑块 */
  input[type='range'] {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 4px;
    margin: 10px 0 2px;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      var(--accent) 0%,
      var(--accent) var(--p, 50%),
      var(--border) var(--p, 50%),
      var(--border) 100%
    );
    outline: none;
    cursor: pointer;
  }
  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: #fff;
    border: 0.5px solid rgba(0, 0, 0, 0.12);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.22);
    transition: transform 0.1s;
  }
  input[type='range']::-webkit-slider-thumb:hover {
    transform: scale(1.12);
  }

  /* 分段控件 */
  .seg {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .seg button {
    flex: 1;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    border-radius: 8px;
    padding: 7px 0;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .seg button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  /* 开关 */
  .toggle {
    width: 42px;
    height: 25px;
    border-radius: 13px;
    border: none;
    background: var(--border);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s;
  }
  .toggle::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 21px;
    height: 21px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: left 0.15s;
  }
  .toggle.on {
    background: var(--accent);
  }
  .toggle.on::after {
    left: 19px;
  }

  .keybtn {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 14px;
    background: var(--accent);
    color: #fff;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin-bottom: 12px;
  }
  .stat {
    background: var(--bg);
    border-radius: 9px;
    padding: 12px 6px;
    text-align: center;
  }
  .stat b {
    display: block;
    font-size: 14.5px;
    font-variant-numeric: tabular-nums;
  }
  .stat span {
    font-size: 10.5px;
    color: var(--muted);
  }
  .path {
    font-size: 10px;
    color: var(--muted);
    background: var(--bg);
    border-radius: 7px;
    padding: 7px 9px;
    margin-bottom: 12px;
    word-break: break-all;
    font-family: ui-monospace, 'SF Mono', monospace;
  }
  .danger {
    border: 1px solid var(--danger);
    color: var(--danger);
    background: transparent;
    border-radius: 8px;
    padding: 7px 14px;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
  }
  .danger:hover {
    background: rgba(255, 59, 48, 0.08);
  }
</style>
