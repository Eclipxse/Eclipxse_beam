import { describe, expect, it } from 'vitest';
import { connectionLabel, formatBytes, normalizePairingCode, shortCode } from './format';

describe('formatBytes', () => {
  it('formats common file sizes', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(10 * 1024 * 1024)).toBe('10 MB');
  });

  it('handles invalid input safely', () => {
    expect(formatBytes(Number.NaN)).toBe('0 B');
    expect(formatBytes(-20)).toBe('0 B');
  });
});

describe('pairing helpers', () => {
  it('reads a peer id from a shared link', () => {
    expect(normalizePairingCode('https://example.com/?peer=moon-123')).toBe('moon-123');
  });

  it('cleans a pasted code', () => {
    expect(normalizePairingCode(' moon 123 ')).toBe('moon123');
  });

  it('shortens long codes without hiding both ends', () => {
    expect(shortCode('1234567890abcdefghijkl')).toBe('12345678…ghijkl');
  });

  it('returns readable connection states', () => {
    expect(connectionLabel('connected')).toBe('Connected');
    expect(connectionLabel('unknown')).toBe('Offline');
  });
});
