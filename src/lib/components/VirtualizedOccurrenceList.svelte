<script lang="ts">
  import type { VirtualItem } from '@tanstack/svelte-virtual';
  import type { Occurrence } from '$lib/types/archive';
  import type { Snippet } from 'svelte';

  interface Props {
    virtualItems: VirtualItem[];
    occurrenceCache: Map<number, Occurrence>;
    occurrenceCacheVersion: number;
    children: Snippet<[{ virtualRow: VirtualItem; occurrence: Occurrence | undefined }]>;
  }

  let { virtualItems, occurrenceCache, occurrenceCacheVersion, children }: Props = $props();
</script>

<!--
  Key each virtual item by both its index and the cache version.
  This forces Svelte to re-render items when the cache is cleared and repopulated
  (e.g., during search or pagination). Without the cache version in the key,
  Svelte would reuse existing DOM elements that reference stale occurrence data,
  causing cards to display incorrect data or not update when search results change.
-->
{#each virtualItems as virtualRow (`${virtualRow.index}-${occurrenceCacheVersion}`)}
  {@const occurrence = occurrenceCache.get(virtualRow.index)}
  {@render children({ virtualRow, occurrence })}
{/each}
