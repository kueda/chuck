import type { Multimedia } from '$lib/types/archive';

/// Returns true if the item represents a sound (audio) media record.
export function isSoundMedia(item: Partial<Multimedia>): boolean {
  return item.type === 'Sound' || (item.format?.startsWith('audio/') ?? false);
}
