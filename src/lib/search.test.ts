import { describe, expect, it } from 'vitest';
import { filterEntries, highlight } from './search';

const items = [
  { id: 1, content: 'Hello World' },
  { id: 2, content: 'git push origin main' },
  { id: 3, content: 'https://github.com/foo' },
];

describe('filterEntries', () => {
  it('matches case-insensitive substring', () => {
    const out = filterEntries(items, 'WORLD');
    expect(out.map((i) => i.id)).toEqual([1]);
  });

  it('empty query returns all', () => {
    expect(filterEntries(items, '')).toHaveLength(3);
  });

  it('no match returns empty', () => {
    expect(filterEntries(items, 'zzz')).toHaveLength(0);
  });
});

describe('highlight', () => {
  it('wraps matched part in <mark>', () => {
    expect(highlight('Hello World', 'world')).toBe('Hello <mark>World</mark>');
  });

  it('no match returns original escaped', () => {
    expect(highlight('<b>', 'x')).toBe('&lt;b&gt;');
  });
});
