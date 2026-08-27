/**
 * 轻量 i18n：零依赖双语字典。
 * 语言在加载时按系统语言检测一次（zh* → 中文，其余 → 英文），
 * 静态网页无路由切换需求，无需响应式 locale。
 * 约定：所有 key 以中文语义命名并保证 zh / en 键集合完全一致（en 用 satisfies 校验）。
 */

export type Locale = 'zh' | 'en';

function detectLocale(): Locale {
  if (typeof navigator === 'undefined') return 'zh';
  return navigator.language?.toLowerCase().startsWith('zh') ? 'zh' : 'en';
}

export const locale: Locale = detectLocale();

const zh = {
  // ---- 主弹窗 ----
  ph_search: '搜索剪贴板…',
  empty_no_match: '无匹配结果',
  empty_history: '暂无剪贴板记录',
  empty_hint: '复制任何内容都会自动出现在这里',
  act_select: '选择',
  act_paste: '粘贴',
  act_pin: '固定',
  act_delete: '删除',
  act_close: '关闭',
  tab_all: '剪贴板',
  tab_pinned: '固定',

  // ---- 设置页导航 ----
  set_general: '常规',
  set_appearance: '外观',
  set_search_position: '搜索与位置',
  set_shortcut: '快捷键',
  set_storage: '存储',
  set_about: '关于',

  // ---- 常规 ----
  autostart: '开机自动启动',
  autostart_hint: '登录系统后自动在后台运行',
  history_limit: '历史记录上限',
  items_count: '{n} 条',
  history_limit_hint: '超过上限自动删除最旧的未固定条目（固定项永不被清理）',
  ax_title: '辅助功能授权（粘贴必需）',
  ax_notice_1:
    '应用已使用固定的签名证书，正常情况下更新后授权会自动保留，无需重复授权。',
  ax_notice_2_prefix: '若更新后粘贴失效，请到 ',
  ax_notice_path: '系统设置 → 隐私与安全性 → 辅助功能',
  ax_notice_2_suffix: '，移除旧的 ClipboardX 条目后重新添加。',

  // ---- 外观 ----
  opacity: '弹窗透明度',
  glass_strength: '高光反光 / 玻璃质感',
  glass_strength_hint: '低 = 柔和哑光 · 高 = 晶莹流光、高光棱线与控件反光更亮',
  win_width: '窗口宽度',
  win_height: '窗口高度',
  mode_auto: '自动',
  mode_custom: '自定义',
  width_auto_hint: '自动：使用默认宽度',
  width_custom_hint: '自定义：{w} px',
  height_auto_hint: '自动：高度随条目数量变化',
  height_custom_hint: '自定义：{h} px',
  thumb_height: '图片预览高度',

  // ---- 搜索与位置 ----
  search_enabled: '启用搜索框',
  search_enabled_hint: '在弹窗顶部显示搜索输入',
  search_mode: '搜索模式',
  search_fuzzy: '模糊匹配',
  search_exact: '精准匹配',
  fuzzy_hint: '模糊：按字符顺序匹配，如「gpo」可匹配「git push origin」',
  position_mode: '弹窗出现位置',
  position_cursor: '跟随鼠标',
  position_center: '屏幕居中',
  cursor_hint: '跟随鼠标时自动避开屏幕边缘',

  // ---- 快捷键 ----
  shortcut_show: '唤起剪贴板',
  shortcut_record_hint: '点击按钮后按下新的组合键',
  recording: '按下组合键…',
  popup_keys_hint: '弹窗内快捷键：↑↓ 选择 · ⏎ 粘贴 · ⌘P 固定 · ⌫ 删除 · Esc 关闭',

  // ---- 存储 ----
  stat_db: '数据库',
  stat_images: '图片缓存',
  stat_entries: '历史条目',
  image_cache_label: '图片缓存',
  files_count: '{size} · {n} 个文件',
  btn_clear_cache: '清除缓存',
  cache_location: '缓存位置',
  clear_history_label: '清空历史记录',
  clear_history_hint: '删除全部未固定条目，固定项会保留',
  btn_clear_history: '清空历史',
  records_label: '剪贴板记录',
  records_open_hint: '点击查看全部剪贴板记录',
  img_entry: '图片',
  btn_view: '查看',
  btn_hide: '收起',
  recent_only: '仅显示最近 {max} 条',
  reading_stats: '读取存储信息中…',
  confirm_clear_history: '确定清空历史记录吗？固定条目会保留。',
  confirm_clear_images: '确定清除图片缓存吗？非固定图片条目会被删除，固定图片会保留。',

  // ---- 状态提示 ----
  st_saved: '已保存',
  st_save_failed: '保存失败',
  st_cleared: '已清空 {n} 条记录',
  st_images_cleared: '图片缓存已清除',
  st_set_failed: '设置失败',

  // ---- 关于 ----
  version_label: '版本 {v}',
  author_line: '作者：Codex 和 DeepSeek v4 flash 0731',
  about_desc: '一个专为 macOS 打造的现代化剪贴板管理器 —— 原生体验、完全离线、低内存。',
  check_update: '检查更新',
  check_update_hint: '自动下载并安装最新版本，安装完成后自动重启',
  checking: '检查中…',
  up_to_date: '已是最新版本（{v}）',
  update_found:
    '发现新版本 {latest}（当前 {current}）。\n是否立即下载并安装？安装完成后应用会自动重启。',
  updating_pct: '正在下载并安装更新… {pct}%',
  updating_bytes: '正在下载并安装更新…（{mb} MB）',
  updating: '正在下载并安装更新…',
} as const;

export type I18nKey = keyof typeof zh;

const en: Record<I18nKey, string> = {
  // ---- Main popup ----
  ph_search: 'Search clipboard…',
  empty_no_match: 'No matches',
  empty_history: 'No clipboard history yet',
  empty_hint: 'Anything you copy shows up here automatically',
  act_select: 'Select',
  act_paste: 'Paste',
  act_pin: 'Pin',
  act_delete: 'Delete',
  act_close: 'Close',
  tab_all: 'History',
  tab_pinned: 'Pinned',

  // ---- Settings nav ----
  set_general: 'General',
  set_appearance: 'Appearance',
  set_search_position: 'Search & Position',
  set_shortcut: 'Shortcut',
  set_storage: 'Storage',
  set_about: 'About',

  // ---- General ----
  autostart: 'Launch at login',
  autostart_hint: 'Run in the background after you log in',
  history_limit: 'History limit',
  items_count: '{n} items',
  history_limit_hint:
    'Oldest unpinned entries are removed beyond the limit (pinned items are never cleaned)',
  ax_title: 'Accessibility permission (required for pasting)',
  ax_notice_1:
    'The app is signed with a stable certificate — permission normally survives updates.',
  ax_notice_2_prefix: 'If pasting stops working after an update, go to ',
  ax_notice_path: 'System Settings → Privacy & Security → Accessibility',
  ax_notice_2_suffix: ', remove the old ClipboardX entry and add it again.',

  // ---- Appearance ----
  opacity: 'Popup opacity',
  glass_strength: 'Glass Specularity / Luster',
  glass_strength_hint: 'Low = subtle matte · High = luminous rim highlight & crystalline luster',
  win_width: 'Window width',
  win_height: 'Window height',
  mode_auto: 'Auto',
  mode_custom: 'Custom',
  width_auto_hint: 'Auto: use the default width',
  width_custom_hint: 'Custom: {w} px',
  height_auto_hint: 'Auto: adapts to the number of entries',
  height_custom_hint: 'Custom: {h} px',
  thumb_height: 'Image preview height',

  // ---- Search & position ----
  search_enabled: 'Enable search box',
  search_enabled_hint: 'Show a search input at the top of the popup',
  search_mode: 'Search mode',
  search_fuzzy: 'Fuzzy',
  search_exact: 'Exact',
  fuzzy_hint:
    'Fuzzy matches characters in order, e.g. "gpo" matches "git push origin"',
  position_mode: 'Popup position',
  position_cursor: 'Follow cursor',
  position_center: 'Screen center',
  cursor_hint: 'Keeps away from screen edges when following the cursor',

  // ---- Shortcut ----
  shortcut_show: 'Show clipboard',
  shortcut_record_hint: 'Click the button, then press the new combination',
  recording: 'Press new keys…',
  popup_keys_hint:
    'In-popup shortcuts: ↑↓ select · ⏎ paste · ⌘P pin · ⌫ delete · Esc close',

  // ---- Storage ----
  stat_db: 'Database',
  stat_images: 'Image cache',
  stat_entries: 'Entries',
  image_cache_label: 'Image cache',
  files_count: '{size} · {n} files',
  btn_clear_cache: 'Clear cache',
  cache_location: 'Location',
  clear_history_label: 'Clear history',
  clear_history_hint: 'Deletes all unpinned entries; pinned items are kept',
  btn_clear_history: 'Clear history',
  records_label: 'Clipboard records',
  records_open_hint: 'Click to browse all clipboard records',
  img_entry: 'Image',
  btn_view: 'View',
  btn_hide: 'Hide',
  recent_only: 'Showing latest {max} only',
  reading_stats: 'Loading storage info…',
  confirm_clear_history:
    'Clear all clipboard history? Pinned entries will be kept.',
  confirm_clear_images:
    'Clear the image cache? Unpinned image entries will be deleted; pinned images are kept.',

  // ---- Status ----
  st_saved: 'Saved',
  st_save_failed: 'Save failed',
  st_cleared: 'Cleared {n} records',
  st_images_cleared: 'Image cache cleared',
  st_set_failed: 'Failed to save',

  // ---- About ----
  version_label: 'Version {v}',
  author_line: 'Author: Codex & DeepSeek v4 flash 0731',
  about_desc:
    'A modern clipboard manager built for macOS — native feel, fully offline, low memory.',
  check_update: 'Check for updates',
  check_update_hint: 'Downloads and installs updates automatically, then restarts',
  checking: 'Checking…',
  up_to_date: 'Up to date ({v})',
  update_found:
    'New version {latest} available (current {current}).\nDownload and install now? The app restarts automatically afterwards.',
  updating_pct: 'Downloading and installing update… {pct}%',
  updating_bytes: 'Downloading and installing update… ({mb} MB)',
  updating: 'Downloading and installing update…',
};

/** 取翻译并用 {name} 占位符做参数替换。 */
export function t(key: I18nKey, vars?: Record<string, string | number>): string {
  const dict = locale === 'en' ? en : zh;
  let s = dict[key];
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}
