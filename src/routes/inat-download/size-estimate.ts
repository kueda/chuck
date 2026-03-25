// Size estimation constants (derived from sample archives with 518 observations)
// Measured compressed bytes per observation:
//   Base: ~79, SimpleMultimedia: ~16, Identifications: ~146
const SIZE_ESTIMATE_SAFETY_MARGIN = 1.2;
export const BYTES_PER_OBSERVATION = 79 * SIZE_ESTIMATE_SAFETY_MARGIN;
export const BYTES_PER_OBSERVATION_MULTIMEDIA =
  16 * SIZE_ESTIMATE_SAFETY_MARGIN;
export const BYTES_PER_OBSERVATION_IDENTIFICATIONS =
  146 * SIZE_ESTIMATE_SAFETY_MARGIN;
export const BYTES_PER_OBSERVATION_COMMENTS = 40 * SIZE_ESTIMATE_SAFETY_MARGIN;
export const BYTES_PER_PHOTO = 1_800_000;
export const BYTES_PER_SOUND = 1_000_000;
export const ONE_GB = 1_000_000_000;

export function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} bytes`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  if (bytes < 1_000_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes < 1_000_000_000_000)
    return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  return `${(bytes / 1_000_000_000_000).toFixed(1)} TB 😱`;
}
