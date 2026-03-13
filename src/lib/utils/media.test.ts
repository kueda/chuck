import { describe, expect, it } from 'vitest';
import { isSoundMedia } from './media';

describe('isSoundMedia', () => {
  it('returns true for type=Sound', () => {
    expect(isSoundMedia({ type: 'Sound', identifier: 'sounds/1.mp3' })).toBe(
      true,
    );
  });

  it('returns true for audio/* format', () => {
    expect(
      isSoundMedia({ format: 'audio/mpeg', identifier: 'sounds/1.mp3' }),
    ).toBe(true);
    expect(
      isSoundMedia({ format: 'audio/ogg', identifier: 'sounds/1.ogg' }),
    ).toBe(true);
  });

  it('returns false for image items', () => {
    expect(
      isSoundMedia({ type: 'StillImage', identifier: 'photos/1.jpg' }),
    ).toBe(false);
    expect(
      isSoundMedia({ format: 'image/jpeg', identifier: 'photos/1.jpg' }),
    ).toBe(false);
  });

  it('returns false for items without audio indicators', () => {
    expect(isSoundMedia({ identifier: 'photos/1.jpg' })).toBe(false);
    expect(isSoundMedia({})).toBe(false);
  });
});
