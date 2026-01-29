<!--
  Logic specific to a virtualized list of occurrences, e.g. rendering
  and delegating click handling to parent
-->

<script lang="ts">
import type { VirtualItem } from '@tanstack/svelte-virtual';
import type { Snippet } from 'svelte';
import type { Occurrence } from '$lib/types/archive';

interface Props {
  virtualItems: VirtualItem[];
  occurrenceCache: Map<number, Occurrence>;
  occurrenceCacheVersion: number;
  coreIdColumn: string;
  handleOccurrenceClick: (occurrence: Occurrence, index: number) => void;
  selectedOccurrenceIndex: number | null;
  children: Snippet<
    [
      {
        virtualRow: VirtualItem;
        occurrence: Occurrence | undefined;
      },
    ]
  >;
}

const {
  virtualItems,
  occurrenceCache,
  occurrenceCacheVersion,
  coreIdColumn,
  handleOccurrenceClick,
  selectedOccurrenceIndex,
  children,
}: Props = $props();
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
  <div
    id={occurrence
      ?`item-${occurrence[coreIdColumn as keyof Occurrence]}`
      : ''
    }
    class="occurrence-list-item"
  >
    {@render children({ virtualRow, occurrence })}
  </div>
{/each}
