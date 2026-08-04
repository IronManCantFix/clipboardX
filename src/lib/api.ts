import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type EntryType = 'text' | 'rtf' | 'image' | 'file' | 'url';

export interface Entry {
  id: number;
  type: EntryType;
  content: string;
  rtf_data: number[] | null;
  image_path: string | null;
  thumb_path: string | null;
  file_paths: string[];
  source_app: string | null;
  pinned: boolean;
  created_at: number;
  last_used_at: number;
}

export interface Settings {
  history_limit: number;
  shortcut: string;
}

export const api = {
  list: (tab?: 'pinned') => invoke<Entry[]>('list_entries', { tab: tab ?? null }),
  search: (query: string) => invoke<Entry[]>('search_entries', { query }),
  setPinned: (id: number, pinned: boolean) => invoke('set_pinned', { id, pinned }),
  delete: (id: number) => invoke('delete_entry', { id }),
  paste: (id: number) => invoke('paste_entry', { id }),
  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (settings: Settings) => invoke('update_settings', { settings }),
};

export function onNewEntry(cb: () => void): Promise<() => void> {
  return listen('clipboard:new-entry', cb);
}
