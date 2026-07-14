import { describe, expect, it } from 'vitest';
import { getChunkRanges } from './transfer';

describe('getChunkRanges', () => {
  it('returns no chunks for an empty file', () => {
    expect(getChunkRanges(0)).toEqual([]);
  });

  it('keeps a small file in one chunk', () => {
    expect(getChunkRanges(120, 64 * 1024)).toEqual([{ start: 0, end: 120 }]);
  });

  it('does not read past the final byte', () => {
    expect(getChunkRanges(10, 4)).toEqual([
      { start: 0, end: 4 },
      { start: 4, end: 8 },
      { start: 8, end: 10 },
    ]);
  });

  it('rejects an invalid chunk size', () => {
    expect(() => getChunkRanges(10, 0)).toThrow('Chunk size must be a positive number.');
  });
});
