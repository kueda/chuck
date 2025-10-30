import { describe, it, expect } from 'vitest';
import { formatETR } from './format-etr';

describe('formatETR', () => {
  it('returns "..." for null input', () => {
    expect(formatETR(null)).toBe('...');
  });

  it('formats seconds correctly', () => {
    expect(formatETR(0)).toBe('0s remaining');
    expect(formatETR(5)).toBe('5s remaining');
    expect(formatETR(30)).toBe('30s remaining');
    expect(formatETR(59)).toBe('59s remaining');
  });

  it('rounds up fractional seconds', () => {
    expect(formatETR(5.1)).toBe('6s remaining');
    expect(formatETR(5.9)).toBe('6s remaining');
  });

  it('formats minutes without seconds when exactly divisible', () => {
    expect(formatETR(60)).toBe('1m remaining');
    expect(formatETR(120)).toBe('2m remaining');
    expect(formatETR(300)).toBe('5m remaining');
  });

  it('formats minutes with seconds when not exactly divisible', () => {
    expect(formatETR(61)).toBe('1m 1s remaining');
    expect(formatETR(90)).toBe('1m 30s remaining');
    expect(formatETR(125)).toBe('2m 5s remaining');
  });

  it('formats hours without minutes when exactly divisible', () => {
    expect(formatETR(3600)).toBe('1h remaining');
    expect(formatETR(7200)).toBe('2h remaining');
  });

  it('formats hours with minutes when not exactly divisible', () => {
    expect(formatETR(3660)).toBe('1h 1m remaining');
    expect(formatETR(3900)).toBe('1h 5m remaining');
    expect(formatETR(5400)).toBe('1h 30m remaining');
  });

  it('omits seconds from hours format even when present', () => {
    expect(formatETR(3661)).toBe('1h 1m remaining');
    expect(formatETR(3730)).toBe('1h 2m remaining');
  });
});
