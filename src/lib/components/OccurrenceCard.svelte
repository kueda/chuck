<script lang="ts">
import { AudioLines, Calendar, MapPin } from 'lucide-svelte';
import type { Occurrence } from '$lib/types/archive';
import { isSoundMedia } from '$lib/utils/media';
import MediaItem from './MediaItem.svelte';

const { occurrence }: { occurrence: Occurrence } = $props();
</script>

<script lang="ts" module>
  export const EST_HEIGHT = 286;
</script>

<div
  class="
    occurrence-item
    occurrence-card
    card
    preset-filled-surface-100-900
    border-surface-200-800
    rounded-md
    border-2
    divide-surface-200-800
    w-full
    divide-y
    flex
    flex-col
    overflow-clip
  "
>
  <header class="rounded-t-sm">
    <div class="h-[200px] preset-filled-surface-200-800 flex justify-center items-center relative">
      <MediaItem
        multimediaItem={occurrence?.multimedia?.find((m) => m.identifier)}
        audiovisualItem={occurrence?.audiovisual?.find(av => av.accessURI)}
        noInteraction
      />
      {#if occurrence?.multimedia?.find(m => isSoundMedia(m)) && occurrence?.multimedia?.find(m => !isSoundMedia(m))}
        <div class="absolute top-0 end-0 preset-filled-surface-200-800 p-2">
          <AudioLines size={16} />
        </div>
      {/if}
    </div>
  </header>
  <article class="space-y-2 p-3">
    <div class="font-medium text-base italic truncate">
      {occurrence.scientificName || 'Unknown'}
    </div>
    {#if occurrence.eventDate}
      <div class="text-sm text-surface-600-400 truncate">
        <div class="flex flex-row gap-1 items-center">
          <Calendar size={16} />
          {occurrence.eventDate}
        </div>
      </div>
    {/if}
  </article>
</div>
