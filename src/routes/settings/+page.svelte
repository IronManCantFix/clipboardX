<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { api, type Settings, type StorageStats } from '../../lib/api';

  let settings = $state<Settings>({
    history_limit: 15,
    shortcut: 'Command+Shift+V',
    opacity: 0.9,
    width_mode: 'auto',
    height_mode: 'auto',
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
  let tab = $state('general');


  const tabs = [
    { id: 'general', label: '常规', icon: 'sliders' },
    { id: 'appearance', label: '外观', icon: 'palette' },
    { id: 'search', label: '搜索与位置', icon: 'search' },
    { id: 'shortcut', label: '快捷键', icon: 'keyboard' },
    { id: 'storage', label: '存储', icon: 'database' },
    { id: 'about', label: '关于', icon: 'info' },
  ];

  const icons: Record<string, string> = {
    sliders: '<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>',
    palette:
      '<circle cx="13.5" cy="6.5" r=".5"/><circle cx="17.5" cy="10.5" r=".5"/><circle cx="8.5" cy="7.5" r=".5"/><circle cx="6.5" cy="12.5" r=".5"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 011.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>',
    search: '<circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/>',
    keyboard:
      '<rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 14h.01M18 14h.01M9 14h6"/>',
    database:
      '<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>',
    info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>',
  };

  onMount(async () => {
    settings = await api.getSettings();
    void api.getAutostart().then((v) => (autostart = v)).catch(() => {});
    void getVersion().then((v) => (version = v)).catch(() => {});
  });

  let storageLoaded = $state(false);
  let entries = $state<import('../../lib/api').Entry[]>([]);
  let historyOpen = $state(false);

  const MAX_DISPLAY = 50;

  const entryIconPaths: Record<string, string> = {
    text: '<path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/><path d="M9 13h6M9 17h4"/>',
    url: '<path d="M10 14l4-4"/><path d="M8 12l-2 2a4 4 0 006 6l2-2"/><path d="M16 12l2-2a4 4 0 00-6-6l-2 2"/>',
    image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M21 16l-5-5-9 9"/>',
    file: '<path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>',
    rtf: '<path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/><path d="M9 11h6M9 15h6M9 19h3"/>',
  };

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

  function switchTab(id: string) {
    tab = id;
    if (id === 'storage' && !storageLoaded) {
      storageLoaded = true;
      void loadStats();
    }
  }

  async function loadStats() {
    stats = await api.getStorageStats();
  }

  let saved = $state(false);

  async function save() {
    try {
      await api.updateSettings(settings);
      status = '已保存';
      saved = true;
      setTimeout(() => (saved = false), 1600);
    } catch (e) {
      status = '保存失败';
      saved = false;
      console.error(e);
    }
  }

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    settings[key] = value;
    void save();
  }

  async function clearHistory() {
    const ok = await confirm('确定清空历史记录吗？固定条目会保留。', {
      title: 'ClipboardX',
      kind: 'warning',
    });
    if (!ok) return;
    const n = await api.clearHistory();
    status = `已清空 ${n} 条记录`;
    entries = [];
    void loadStats();
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
    const ok = await confirm('确定清除全部图片缓存吗？图片历史条目也会被删除。', {
      title: 'ClipboardX',
      kind: 'warning',
    });
    if (!ok) return;
    await api.clearImages();
    status = '图片缓存已清除';
    void loadStats();
  }

  async function checkUpdate() {
    checking = true;
    updateMsg = '';
    let updateUrl: string | null = null;
    try {
      const info = await api.checkForUpdates();
      if (!info.has_update) {
        updateMsg = `已是最新版本（${info.current_version}）`;
        return;
      }
      updateUrl = info.url ?? null;
      updateMsg = `发现新版本 ${info.latest_version}（当前 ${info.current_version}）`;
    } catch (e) {
      updateMsg = String(e).replace(/^Error:\s*/, '');
    } finally {
      checking = false;
      // 检测到新版本时自动打开 Release 页面
      if (updateUrl) {
        void openUrl(updateUrl);
      }
    }
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
  <aside class="sidebar">
    <div class="brand">
      <img class="logo" src="/icon.png" alt="ClipboardX" />
      <div>
        <b>ClipboardX</b>
        <span>v{version || '…'}</span>
      </div>
    </div>
    <nav>
      {#each tabs as t (t.id)}
        <button class:active={tab === t.id} onclick={() => switchTab(t.id)} aria-label={t.label}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"
               stroke-linecap="round" stroke-linejoin="round">
            {@html icons[t.icon]}
          </svg>
          <span>{t.label}</span>
        </button>
      {/each}
    </nav>
    <div class="foot-status" class:ok={saved}>{status || '\u00a0'}</div>
  </aside>

  <main class="panel">
    <div hidden={tab !== "general"}>
      <section>
        <h2>常规</h2>
        <div class="card">
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
          <div class="divider"></div>
          <div class="field">
            <div class="row-top">
              <span class="lbl">历史记录上限</span>
              <span class="val">{settings.history_limit} 条</span>
            </div>
            <input type="range" min="1" max="100" value={settings.history_limit}
                   style="--p:{pct(settings.history_limit, 1, 100)}%"
                   oninput={(e) => set('history_limit', Number((e.target as HTMLInputElement).value))} />
            <p class="hint">超过上限自动删除最旧的未固定条目（固定项永不被清理）</p>
          </div>
        </div>
      </section>

    </div>
    <div hidden={tab !== "appearance"}>
      <section>
        <h2>外观</h2>
        <div class="card">
          <div class="field">
            <div class="row-top">
              <span class="lbl">弹窗透明度</span>
              <span class="val">{Math.round(settings.opacity * 100)}%</span>
            </div>
            <input type="range" min="0.3" max="1" step="0.05" value={settings.opacity}
                   style="--p:{pct(settings.opacity, 0.3, 1)}%"
                   oninput={(e) => set('opacity', Number((e.target as HTMLInputElement).value))} />
          </div>
          <div class="divider"></div>
          <div class="field">
            <div class="row-top"><span class="lbl">窗口宽度</span></div>
            <div class="seg">
              <button class:on={settings.width_mode === 'auto'} onclick={() => set('width_mode', 'auto')}>自动</button>
              <button class:on={settings.width_mode === 'custom'} onclick={() => set('width_mode', 'custom')}>自定义</button>
            </div>
            {#if settings.width_mode === 'custom'}
              <div class="sub">
                <input type="range" min="220" max="800" step="10" value={settings.window_width}
                       style="--p:{pct(settings.window_width, 220, 800)}%"
                       oninput={(e) => set('window_width', Number((e.target as HTMLInputElement).value))} />
              </div>
            {/if}
            <p class="hint">{settings.width_mode === 'auto' ? '自动：使用默认宽度' : `自定义：${settings.window_width} px`}</p>
          </div>
          <div class="divider"></div>
          <div class="field">
            <div class="row-top"><span class="lbl">窗口高度</span></div>
            <div class="seg">
              <button class:on={settings.height_mode === 'auto'} onclick={() => set('height_mode', 'auto')}>自动</button>
              <button class:on={settings.height_mode === 'custom'} onclick={() => set('height_mode', 'custom')}>自定义</button>
            </div>
            {#if settings.height_mode === 'custom'}
              <div class="sub">
                <input type="range" min="200" max="1200" step="10" value={settings.window_height}
                       style="--p:{pct(settings.window_height, 200, 1200)}%"
                       oninput={(e) => set('window_height', Number((e.target as HTMLInputElement).value))} />
              </div>
            {/if}
            <p class="hint">{settings.height_mode === 'auto' ? '自动：高度随条目数量变化' : `自定义：${settings.window_height} px`}</p>
          </div>
          <div class="divider"></div>
          <div class="field">
            <div class="row-top">
              <span class="lbl">图片预览高度</span>
              <span class="val">{settings.image_preview_height} px</span>
            </div>
            <input type="range" min="24" max="200" step="4" value={settings.image_preview_height}
                   style="--p:{pct(settings.image_preview_height, 24, 200)}%"
                   oninput={(e) => set('image_preview_height', Number((e.target as HTMLInputElement).value))} />
          </div>
        </div>
      </section>

    </div>
    <div hidden={tab !== "search"}>
      <section>
        <h2>搜索与位置</h2>
        <div class="card">
          <div class="field row">
            <div>
              <span class="lbl">启用搜索框</span>
              <p class="hint">在弹窗顶部显示搜索输入</p>
            </div>
            <button class="toggle" class:on={settings.search_enabled} aria-label="启用搜索框"
                    onclick={() => set('search_enabled', !settings.search_enabled)}></button>
          </div>
          <div class="divider"></div>
          <div class="field">
            <div class="row-top"><span class="lbl">搜索模式</span></div>
            <div class="seg">
              <button class:on={settings.search_mode === 'fuzzy'} onclick={() => set('search_mode', 'fuzzy')}>模糊匹配</button>
              <button class:on={settings.search_mode === 'exact'} onclick={() => set('search_mode', 'exact')}>精准匹配</button>
            </div>
            <p class="hint">模糊：按字符顺序匹配，如「gpo」可匹配「git push origin」</p>
          </div>
          <div class="divider"></div>
          <div class="field">
            <div class="row-top"><span class="lbl">弹窗出现位置</span></div>
            <div class="seg">
              <button class:on={settings.position_mode === 'cursor'} onclick={() => set('position_mode', 'cursor')}>跟随鼠标</button>
              <button class:on={settings.position_mode === 'center'} onclick={() => set('position_mode', 'center')}>屏幕居中</button>
            </div>
            <p class="hint">跟随鼠标时自动避开屏幕边缘</p>
          </div>
        </div>
      </section>

    </div>
    <div hidden={tab !== "shortcut"}>
      <section>
        <h2>快捷键</h2>
        <div class="card">
          <div class="field row">
            <div>
              <span class="lbl">唤起剪贴板</span>
              <p class="hint">点击按钮后按下新的组合键</p>
            </div>
            <button class="keybtn" onclick={recording ? undefined : startRecord}>
              {recording ? '按下组合键…' : settings.shortcut}
            </button>
          </div>
          <div class="divider"></div>
          <div class="field">
            <p class="hint">弹窗内快捷键：↑↓ 选择 · ⏎ 粘贴 · ⌘P 固定 · ⌫ 删除 · Esc 关闭</p>
          </div>
        </div>
      </section>

    </div>
    <div hidden={tab !== "storage"}>
      <section class="storage-section">
        <h2>存储</h2>
        {#if stats}
          <div class="stat-grid">
            <div class="stat"><b>{fmtSize(stats.db_size)}</b><span>数据库</span></div>
            <div class="stat"><b>{fmtSize(stats.images_size)}</b><span>图片缓存</span></div>
            <div class="stat"><b>{stats.entry_count}</b><span>历史条目</span></div>
          </div>

          <div class="card storage-card">
            <div class="storage-row">
              <div class="storage-info">
                <span class="lbl">图片缓存</span>
                <span class="hint">{fmtSize(stats.images_size)} · {stats.image_count} 个文件</span>
              </div>
              <button class="danger" onclick={clearImages}>清除缓存</button>
            </div>
            <div class="divider"></div>
            <div class="storage-path">
              <span class="hint">缓存位置</span>
              <div class="path" title={stats.images_dir}>{stats.images_dir}</div>
            </div>
          </div>

          <div class="card storage-card">
            <div class="storage-row">
              <div class="storage-info">
                <span class="lbl">清空历史记录</span>
                <span class="hint">删除全部未固定条目，固定项会保留</span>
              </div>
              <button class="danger" onclick={clearHistory}>清空历史</button>
            </div>
          </div>

          <div class="card storage-card">
            <div class="storage-row">
              <div class="storage-info">
                <span class="lbl">剪贴板记录</span>
                <span class="hint">{historyOpen ? `${entries.length} 条` : '点击查看全部剪贴板记录'}</span>
              </div>
              <button class="keybtn" onclick={toggleHistory}>{historyOpen ? '收起' : '查看'}</button>
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
                      {e.type === 'image' ? '图片' : e.content}
                    </span>
                    {#if e.pinned}
                      <svg class="e-pin" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M16 3l5 5-4 1-3 3 1 6-2 2-4-5-5 4-1-1 4-5-5-4 2-2 6 1 3-3 1-4z"/>
                      </svg>
                    {/if}
                    <button class="e-del" aria-label="删除" title="删除"
                            onclick={() => deleteOne(e.id)}>
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                           stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
                    </button>
                  </div>
                {:else}
                  <p class="hint" style="text-align:center;padding:12px 0">暂无剪贴板记录</p>
                {/each}
              </div>
              {#if entries.length > MAX_DISPLAY}
                <p class="hint" style="text-align:center;padding-top:6px">仅显示最近 {MAX_DISPLAY} 条</p>
              {/if}
            {/if}
          </div>
        {:else}
          <p class="hint" style="padding:20px 0;text-align:center">读取存储信息中…</p>
        {/if}
      </section>

    </div>
    <div hidden={tab !== "about"}>
      <section>
        <h2>关于</h2>
        <div class="about-card">
          <img class="about-logo" src="/icon.png" alt="ClipboardX" />
          <b>ClipboardX</b>
          <span class="ver">版本 {version || '…'}</span>
          <span class="author">作者：Codex 和 DeepSeek v4 flash 0731</span>
          <p class="about-desc">一个专为 macOS 打造的现代化剪贴板管理器 —— 原生体验、完全离线、低内存。</p>
          <div class="about-links">
            <button class="gh-link" onclick={() => openUrl('https://github.com/IronManCantFix/clipboardX')}>GitHub ↗</button>
          </div>
        </div>
        <div class="card">
          <div class="field row">
            <div>
              <span class="lbl">检查更新</span>
              <p class="hint">从 GitHub Releases 获取最新版本</p>
            </div>
            <button class="keybtn" onclick={checkUpdate} disabled={checking}>
              {checking ? '检查中…' : '检查更新'}
            </button>
          </div>
          {#if updateMsg}
            <p class="hint update-msg">{updateMsg}</p>
          {/if}
        </div>
      </section>
    </div>
  </main>
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
    --muted: #6e6e73;
    --faint: #98989d;
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
    display: flex;
    height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif;
    color: var(--text);
    font-size: 13px;
    overflow: hidden;
  }

  /* 左侧栏 */
  .sidebar {
    width: 168px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    padding: 18px 10px 12px;
    background: var(--bg);
    border-right: 1px solid var(--border);
    box-sizing: border-box;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 2px 8px 16px;
  }
  .logo {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    object-fit: cover;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
  }
  .brand b {
    display: block;
    font-size: 13.5px;
    line-height: 1.2;
  }
  .brand span {
    font-size: 10.5px;
    color: var(--muted);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 12.5px;
    padding: 7px 9px;
    border-radius: 7px;
    cursor: pointer;
    transition: background 0.15s;
    text-align: left;
  }
  nav button svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    color: var(--muted);
    transition: color 0.15s;
  }
  nav button:hover {
    background: rgba(120, 120, 128, 0.12);
  }
  nav button.active {
    background: rgba(10, 132, 255, 0.14);
    color: var(--text);
    font-weight: 600;
  }
  nav button.active svg {
    color: var(--accent);
  }
  .foot-status {
    margin-top: auto;
    font-size: 10.5px;
    color: var(--muted);
    padding: 8px 8px 0;
    transition: color 0.2s;
  }
  .foot-status.ok {
    color: #34c759;
    font-weight: 600;
  }

  /* 右侧面板 */
  .panel {
    flex: 1;
    overflow-y: auto;
    padding: 26px 30px 40px;
    box-sizing: border-box;
  }
  section h2 {
    font-size: 19px;
    font-weight: 700;
    letter-spacing: -0.01em;
    margin: 0 0 14px;
  }
  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 18px 20px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  }
  .divider {
    height: 1px;
    background: var(--border);
    margin: 16px 0;
  }
  .field {
    margin-bottom: 4px;
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
    margin-bottom: 2px;
  }
  .lbl {
    font-weight: 500;
  }
  .val {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--faint);
    font-size: 11.5px;
    margin: 6px 0 0;
    line-height: 1.45;
  }
  .sub {
    margin-top: 14px;
    padding: 12px 14px;
    background: var(--bg);
    border-radius: 9px;
  }

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
    transition: transform 0.12s;
  }
  input[type='range']::-webkit-slider-thumb:hover {
    transform: scale(1.12);
  }

  .seg {
    display: flex;
    gap: 6px;
    margin-top: 10px;
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
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .seg button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .toggle {
    width: 42px;
    height: 25px;
    border-radius: 13px;
    border: none;
    background: var(--border);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.18s;
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
    transition: left 0.18s;
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
    transition: filter 0.15s;
  }
  .keybtn:hover {
    filter: brightness(1.08);
  }
  .keybtn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-bottom: 14px;
  }
  .stat {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 11px;
    padding: 14px 6px;
    text-align: center;
  }
  .stat b {
    display: block;
    font-size: 15px;
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
    padding: 8px 10px;
    margin-top: 8px;
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
    transition: background 0.15s;
  }
  .danger:hover {
    background: rgba(255, 59, 48, 0.08);
  }

  .storage-section .card + .card {
    margin-top: 12px;
  }
  .storage-card {
    padding: 16px 18px;
  }
  .storage-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .storage-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .storage-info .lbl {
    font-weight: 500;
    font-size: 13px;
  }
  .storage-info .hint {
    margin: 0;
  }
  .storage-path {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .about-card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 26px 20px;
    text-align: center;
    margin-bottom: 14px;
  }
  .about-logo {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    object-fit: cover;
    margin: 0 auto 12px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.22);
  }
  .about-card b {
    display: block;
    font-size: 16px;
  }
  .ver {
    display: block;
    color: var(--muted);
    font-size: 11.5px;
    margin-top: 2px;
  }
  .author {
    display: block;
    color: var(--muted);
    font-size: 11.5px;
    margin-top: 6px;
  }
  .about-desc {
    color: var(--muted);
    font-size: 12px;
    margin: 10px 0 12px;
    line-height: 1.5;
  }
  .about-links .gh-link {
    border: none;
    background: transparent;
    color: var(--accent);
    font-size: 12.5px;
    cursor: pointer;
    padding: 4px 10px;
    border-radius: 7px;
    transition: background 0.15s;
  }
  .about-links .gh-link:hover {
    background: rgba(10, 132, 255, 0.12);
  }
  .entry-list {
    margin-top: 12px;
    max-height: 300px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px;
    border-radius: 7px;
    transition: background 0.12s;
  }
  .entry:hover {
    background: var(--bg);
  }
  .e-ico {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
  }
  .e-text { color: #0a84ff; }
  .e-url { color: #34c759; }
  .e-image { color: #bf5af2; }
  .e-file { color: #ff9f0a; }
  .e-rtf { color: #64d2ff; }
  .e-txt {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12.5px;
  }
  .e-pin {
    width: 11px;
    height: 11px;
    flex-shrink: 0;
    color: #ff9f0a;
  }
  .e-del {
    border: none;
    background: transparent;
    color: var(--faint);
    width: 22px;
    height: 22px;
    border-radius: 5px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.12s, background 0.12s, color 0.12s;
  }
  .entry:hover .e-del {
    opacity: 1;
  }
  .e-del:hover {
    color: var(--danger);
    background: rgba(255, 59, 48, 0.1);
  }
  .e-del svg {
    width: 12px;
    height: 12px;
  }

  .update-msg {
    margin-top: 12px;
    padding: 8px 12px;
    background: var(--bg);
    border-radius: 7px;
  }
</style>
