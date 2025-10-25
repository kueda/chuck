<!--
  Logic specific to a virtualized list of occurrences, e.g. prev/next
  navigation and re-rendering
-->

<script lang="ts">
  import type { VirtualItem } from '@tanstack/svelte-virtual';
  import type { Occurrence } from '$lib/types/archive';
  import type { Snippet } from 'svelte';
  import OccurrenceDrawer from './OccurrenceDrawer.svelte';

  interface Props {
    virtualItems: VirtualItem[];
    occurrenceCache: Map<number, Occurrence>;
    occurrenceCacheVersion: number;
    coreIdColumn: string;
    count: number;
    scrollToIndex?: (index: number, options?: { align?: 'start' | 'center' | 'end' | 'auto' }) => void;
    children: Snippet<[{
      virtualRow: VirtualItem;
      occurrence: Occurrence | undefined;
      handleOccurrenceClick: (occurrence: Occurrence, index: number) => void;
      selectedOccurrenceIndex: string | number | null;
    }]>;
  }

  let {
    virtualItems,
    occurrenceCache,
    occurrenceCacheVersion,
    coreIdColumn,
    count,
    scrollToIndex,
    children
  }: Props = $props();

  let drawerOpen = $state(false);

  // Value of the coreIdColumn for the currently-selected occurrence, passed
  // to the occurrence drawer to show occurrence details
  let selectedOccurrenceId = $state<string | number | null>(null);

  // Index of the currently-selected occurrence in the virtualizer results
  let selectedOccurrenceIndex = $state<number | null>(null);

  function handleOccurrenceClick(occurrence: Occurrence, index: number) {
    const value = occurrence[coreIdColumn as keyof Occurrence];
    selectedOccurrenceId = (typeof value === 'string' || typeof value === 'number') ? value : null;
    selectedOccurrenceIndex = index;
    drawerOpen = true;
  }

  function handlePrevious() {
    if (selectedOccurrenceIndex !== null && selectedOccurrenceIndex > 0) {
      const newIndex = selectedOccurrenceIndex - 1;
      const prevOccurrence = occurrenceCache.get(newIndex);
      if (prevOccurrence) {
        const value = prevOccurrence[coreIdColumn as keyof Occurrence];
        selectedOccurrenceId = (typeof value === 'string' || typeof value === 'number') ? value : null;
        selectedOccurrenceIndex = newIndex;
      }
      if (scrollToIndex) scrollToIndex(selectedOccurrenceIndex, { align: 'auto' });
    }
  }

  function handleNext() {
    if (selectedOccurrenceIndex !== null && selectedOccurrenceIndex < count - 1) {
      const newIndex = selectedOccurrenceIndex + 1;
      const nextOccurrence = occurrenceCache.get(newIndex);
      if (nextOccurrence) {
        const value = nextOccurrence[coreIdColumn as keyof Occurrence];
        selectedOccurrenceId = (typeof value === 'string' || typeof value === 'number') ? value : null;
        selectedOccurrenceIndex = newIndex;
      }
      // For some reason it doesn't actually put selectedOccurrenceIndex on
      // screen when it's the last thing on screen
      if (scrollToIndex) scrollToIndex(selectedOccurrenceIndex + 1, { align: 'auto' });
    }
  }

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
  {@render children({ virtualRow, occurrence, handleOccurrenceClick, selectedOccurrenceIndex })}
{/each}

<OccurrenceDrawer
  bind:open={drawerOpen}
  occurrenceId={selectedOccurrenceId}
  {coreIdColumn}
  onClose={() => { drawerOpen = false; }}
  onPrevious={selectedOccurrenceIndex !== null && selectedOccurrenceIndex > 0 ? handlePrevious : undefined}
  onNext={selectedOccurrenceIndex !== null && selectedOccurrenceIndex < count - 1 ? handleNext : undefined}
/>
