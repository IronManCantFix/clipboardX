export interface Searchable {
  content: string;
}

export type SearchMode = 'fuzzy' | 'exact';

// 内容小写缓存：长文本（大段代码、日志）每次按键重复 toLowerCase 开销可观，
// 以原始字符串为 key 缓存结果；条目数 ≤100 且键值即内容本身， Map 容量上限兜底防膨胀。
const lcCache = new Map<string, string>();
const LC_CACHE_MAX = 1024;

function cachedLower(s: string): string {
  const hit = lcCache.get(s);
  if (hit !== undefined) return hit;
  const v = s.toLowerCase();
  if (lcCache.size >= LC_CACHE_MAX) lcCache.clear();
  lcCache.set(s, v);
  return v;
}

/** 字符顺序模糊匹配（不要求连续）。 */
export function fuzzyMatch(text: string, query: string): boolean {
  const t = cachedLower(text);
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
    const t = cachedLower(i.content);
    return mode === 'exact' ? t.includes(q) : fuzzyMatch(t, q);
  });
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

/** 模糊模式：在原始文本上按字符顺序定位每个命中字符，返回其下标集合。 */
function fuzzyMatchPositions(text: string, query: string): Set<number> | null {
  const t = cachedLower(text);
  const q = query.trim().toLowerCase();
  if (!q) return null;
  let from = 0;
  const positions = new Set<number>();
  for (const ch of q) {
    const i = t.indexOf(ch, from);
    if (i < 0) return null;
    positions.add(i);
    from = i + 1;
  }
  return positions;
}

export function highlight(text: string, query: string): string {
  const q = query.trim().toLowerCase();
  if (!q) return escapeHtml(text);
  // 连续子串：直接高亮整段
  const idx = cachedLower(text).indexOf(q);
  if (idx >= 0) {
    return (
      escapeHtml(text.slice(0, idx)) +
      '<mark>' +
      escapeHtml(text.slice(idx, idx + q.length)) +
      '</mark>' +
      escapeHtml(text.slice(idx + q.length))
    );
  }
  // 模糊匹配：逐字符标记命中位置
  const positions = fuzzyMatchPositions(text, q);
  if (!positions) return escapeHtml(text);
  let out = '';
  let inMark = false;
  for (let i = 0; i < text.length; i++) {
    const hit = positions.has(i);
    if (hit && !inMark) {
      out += '<mark>';
      inMark = true;
    }
    if (!hit && inMark) {
      out += '</mark>';
      inMark = false;
    }
    out += escapeHtml(text[i]);
  }
  if (inMark) out += '</mark>';
  return out;
}
