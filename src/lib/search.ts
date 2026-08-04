export interface Searchable {
  content: string;
}

export type SearchMode = 'fuzzy' | 'exact';

/** 字符顺序模糊匹配（不要求连续）。 */
export function fuzzyMatch(text: string, query: string): boolean {
  const t = text.toLowerCase();
  const q = query.trim().toLowerCase();
  if (!q) return true;
  let from = 0;
  for (const ch of q) {
    const idx = t.indexOf(ch, from);
    if (idx < 0) return false;
    from = idx + 1;
  }
  return true;
}

export function filterEntries<T extends Searchable>(
  items: T[],
  query: string,
  mode: SearchMode = 'fuzzy',
): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return items;
  return items.filter((i) => {
    const t = i.content.toLowerCase();
    return mode === 'exact' ? t.includes(q) : fuzzyMatch(t, q);
  });
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export function highlight(text: string, query: string): string {
  const q = query.trim().toLowerCase();
  const escaped = escapeHtml(text);
  if (!q) return escaped;
  const idx = escaped.toLowerCase().indexOf(q);
  if (idx < 0) return escaped;
  return (
    escaped.slice(0, idx) +
    '<mark>' +
    escaped.slice(idx, idx + q.length) +
    '</mark>' +
    escaped.slice(idx + q.length)
  );
}
