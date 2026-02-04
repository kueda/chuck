import { describe, expect, it } from 'vitest';
import { formatETR } from './format-etr';

describe('formatETR', () => {
  it('returns "..." for null input', () => {
    expect(formatETR(null)).toBe('...');
  });

  it('formats seconds correctly when under 1 minute', () => {
    expect(formatETR(0)).toBe('0s remaining');
    expect(formatETR(5)).toBe('5s remaining');
    expect(formatETR(30)).toBe('30s remaining');
    expect(formatETR(59)).toBe('59s remaining');
  });

  it('rounds up fractional seconds when under 1 minute', () => {
    expect(formatETR(5.1)).toBe('6s remaining');
    expect(formatETR(5.9)).toBe('6s remaining');
  });

  it('rounds up to the minute when 60 seconds or more', () => {
    expect(formatETR(60)).toBe('~1m remaining');
    expect(formatETR(61)).toBe('~2m remaining');
    expect(formatETR(90)).toBe('~2m remaining');
    expect(formatETR(119)).toBe('~2m remaining');
    expect(formatETR(120)).toBe('~2m remaining');
    expect(formatETR(121)).toBe('~3m remaining');
    expect(formatETR(300)).toBe('~5m remaining');
  });

  it('formats hours with minutes rounded up', () => {
    expect(formatETR(3600)).toBe('~1h remaining');
    expect(formatETR(3601)).toBe('~1h 1m remaining');
    expect(formatETR(3660)).toBe('~1h 1m remaining');
    expect(formatETR(3661)).toBe('~1h 2m remaining');
    expect(formatETR(3900)).toBe('~1h 5m remaining');
    expect(formatETR(5400)).toBe('~1h 30m remaining');
    expect(formatETR(7200)).toBe('~2h remaining');
  });
});
