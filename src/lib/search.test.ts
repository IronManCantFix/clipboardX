import { describe, expect, it } from 'vitest';
import { filterEntries, fuzzyMatch, highlight } from './search';

const items = [
  { id: 1, content: 'Hello World' },
  { id: 2, content: 'git push origin main' },
  { id: 3, content: 'https://github.com/foo' },
];

describe('filterEntries', () => {
  it('fuzzy matches in order', () => {
    expect(filterEntries(items, 'hlo').map((i) => i.id)).toEqual([1]);
  });

  it('fuzzy matches across words', () => {
    expect(filterEntries(items, 'gpo').map((i) => i.id)).toEqual([2]);
  });

  it('exact mode requires contiguous substring', () => {
    const out = filterEntries(items, 'hlo', 'exact');
    expect(out).toHaveLength(0);
    expect(filterEntries(items, 'WORLD', 'exact').map((i) => i.id)).toEqual([1]);
  });

  it('empty query returns all', () => {
    expect(filterEntries(items, '')).toHaveLength(3);
  });
});

describe('fuzzyMatch', () => {
  it('matches characters in order', () => {
    expect(fuzzyMatch('hello world', 'hlo')).toBe(true);
    expect(fuzzyMatch('hello world', 'wd')).toBe(true);
    expect(fuzzyMatch('hello world', 'zz')).toBe(false);
    expect(fuzzyMatch('hello world', 'ohl')).toBe(false);
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
