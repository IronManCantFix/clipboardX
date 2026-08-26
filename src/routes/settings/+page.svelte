<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { api, type SettingSetter, type Settings } from '../../lib/api';
  import { t } from '../../lib/i18n';
  import './panel.css';
  import GeneralTab from './components/GeneralTab.svelte';
  import AppearanceTab from './components/AppearanceTab.svelte';
  import SearchPositionTab from './components/SearchPositionTab.svelte';
  import ShortcutTab from './components/ShortcutTab.svelte';
  import StorageTab from './components/StorageTab.svelte';
  import AboutTab from './components/AboutTab.svelte';

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
  let autostart = $state(false);
  let version = $state('');
  let status = $state('');
  let saved = $state(false);
  let tab = $state('general');

  const tabs = [
    { id: 'general', labelKey: 'set_general' as const, icon: 'sliders' },
    { id: 'appearance', labelKey: 'set_appearance' as const, icon: 'palette' },
    { id: 'search', labelKey: 'set_search_position' as const, icon: 'search' },
    { id: 'shortcut', labelKey: 'set_shortcut' as const, icon: 'keyboard' },
    { id: 'storage', labelKey: 'set_storage' as const, icon: 'database' },
    { id: 'about', labelKey: 'set_about' as const, icon: 'info' },
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

  onMount(() => {
    void (async () => {
      settings = await api.getSettings();
      void api.getAutostart().then((v) => (autostart = v)).catch(() => {});
      void getVersion().then((v) => (version = v)).catch(() => {});
    })();
  });

  function setStatus(msg: string) {
    status = msg;
  }

  async function save() {
    try {
      await api.updateSettings(settings);
      status = t('st_saved');
      saved = true;
      setTimeout(() => (saved = false), 1600);
    } catch (e) {
      status = t('st_save_failed');
      saved = false;
      console.error(e);
    }
  }

  // 滑块拖动时高频触发，防抖到停止操作后统一保存
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  const set: SettingSetter = (key, value) => {
    settings[key] = value;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void save(), 300);
  };

  function toggleAutostart() {
    autostart = !autostart;
    void api.setAutostart(autostart).catch(() => {
      autostart = !autostart;
      status = t('st_set_failed');
    });
  }

  function switchTab(id: string) {
    tab = id;
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
      {#each tabs as tb (tb.id)}
        <button class:active={tab === tb.id} onclick={() => switchTab(tb.id)} aria-label={t(tb.labelKey)}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"
               stroke-linecap="round" stroke-linejoin="round">
            {@html icons[tb.icon]}
          </svg>
          <span>{t(tb.labelKey)}</span>
        </button>
      {/each}
    </nav>
    <div class="foot-status" class:ok={saved}>{status || '\u00a0'}</div>
  </aside>

  <main class="panel">
    <div hidden={tab !== 'general'}>
      <GeneralTab {settings} setter={set} {autostart} onToggleAutostart={toggleAutostart} />
    </div>
    <div hidden={tab !== 'appearance'}>
      <AppearanceTab {settings} setter={set} />
    </div>
    <div hidden={tab !== 'search'}>
      <SearchPositionTab {settings} setter={set} />
    </div>
    <div hidden={tab !== 'shortcut'}>
      <ShortcutTab {settings} setter={set} />
    </div>
    <div hidden={tab !== 'storage'}>
      <StorageTab active={tab === 'storage'} onStatus={setStatus} />
    </div>
    <div hidden={tab !== 'about'}>
      <AboutTab {version} />
    </div>
  </main>
</div>
