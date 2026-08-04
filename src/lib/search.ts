export interface Searchable {
  content: string;
}

export function filterEntries<T extends Searchable>(items: T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return items;
  return items.filter((i) => i.content.toLowerCase().includes(q));
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
