export function formatETR(seconds: number | null): string {
  if (seconds === null) return '...';

  const rounded = Math.ceil(seconds);

  if (rounded < 60) {
    return `${rounded}s remaining`;
  } else if (rounded < 3600) {
    const minutes = Math.ceil(rounded / 60);
    return `~${minutes}m remaining`;
  } else {
    const hours = Math.floor(rounded / 3600);
    const remainingSeconds = rounded % 3600;
    const minutes = Math.ceil(remainingSeconds / 60);
    return minutes > 0
      ? `~${hours}h ${minutes}m remaining`
      : `~${hours}h remaining`;
  }
}
