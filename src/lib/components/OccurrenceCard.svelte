<script lang="ts">
  import type { Occurrence } from '$lib/types/archive';
  import MediaItem from './MediaItem.svelte';

  const { occurrence }: { occurrence: Occurrence } = $props();
  const mediaItem = occurrence?.multimedia?.find(m => m.identifier);
</script>

<script lang="ts" module>
  export const EST_HEIGHT = 286;
</script>

<div
  class="occurrence-item occurrence-card card preset-filled-surface-100-900 border-surface-200-800
  rounded-md border-2 divide-surface-200-800 w-full divide-y flex flex-col"
>
  <header class="rounded-t-sm">
    <div class="h-[200px] preset-filled-surface-200-800 flex justify-center items-center relative">
      <MediaItem media={mediaItem} />
    </div>
  </header>
  <article class="space-y-2 p-3">
    <div class="font-medium text-base italic truncate">
      {occurrence.scientificName || 'Unknown'}
    </div>
    {#if occurrence.eventDate}
      <div class="text-sm text-surface-600-400 truncate">
        <span class="font-semibold">Date:</span> {occurrence.eventDate}
      </div>
    {/if}
  </article>
</div>
