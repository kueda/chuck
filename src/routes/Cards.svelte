<script lang="ts">
  import VirtualizedList from '$lib/components/VirtualizedList.svelte';
  import VirtualizedOccurrenceList from '$lib/components/VirtualizedOccurrenceList.svelte';
  import type {
    VirtualListData,
    Props as VirtualizedListProps
  } from '$lib/components/VirtualizedList.svelte';
  import type { Occurrence } from '$lib/types/archive';
  import OccurrenceCard, { EST_HEIGHT } from '$lib/components/OccurrenceCard.svelte';

  interface Props extends Pick<VirtualizedListProps, 'count' | 'scrollElement' | 'onVisibleRangeChange'> {
    occurrenceCache: Map<number, Occurrence>;
    occurrenceCacheVersion: number;
    coreIdColumn: string;
  }

  let {
    occurrenceCache,
    occurrenceCacheVersion,
    count,
    scrollElement,
    onVisibleRangeChange,
    coreIdColumn
  }: Props = $props();

  // Tanstack Virtual needs to now whether the virtualized items are arranged
  // in columns (which it calls lanes) and if so how many, so we're
  // recreating tailwind's breakpoints here to ensure the virtualizer knows
  // how many columns there are and that it gets recreated when the number of
  // columns change
  let windowWidth = $state(0);
  const lanes = $derived.by(() => {
    if (windowWidth >= 1536) return 6;
    if (windowWidth >= 1280) return 5;
    if (windowWidth >= 1024) return 4;
    return 3;
  });

  // Track window width so we can responsively tell the virtualizer how many
  // columns there are, which allows it to calculate heights correctly
  $effect(() => {
    const updateWidth = () => {
      windowWidth = window.innerWidth;
    };

    // Set initial width
    updateWidth();

    // Listen for window resize events
    window.addEventListener('resize', updateWidth);

    return () => {
      window.removeEventListener('resize', updateWidth);
    };
  });
</script>

<!--
  Force VirtualizedList to remount when count or cache changes.
  This solves a Svelte 5 reactivity issue where TanStack Virtual's virtualizer
  doesn't properly recreate when count changes rapidly (e.g., 1000 → 0 → 5 during search).
  Without this, the virtualizer renders virtual items based on stale count values,
  showing "Loading..." placeholders for items that don't exist.
-->
{#key `${count}-${occurrenceCacheVersion}`}
<VirtualizedList
  {count}
  {scrollElement}
  estimateSize={EST_HEIGHT}
  {lanes}
  {onVisibleRangeChange}
>
  {#snippet children(data: VirtualListData)}
    <!--
      Force re-render when virtualizer recreates (tracked by data._key).
      This ensures the DOM structure is rebuilt when the virtualizer changes,
      preventing issues with heights not being reset properly when switching
      between views with different item sizes (e.g., table rows vs cards).
    -->
    {#key data._key}
      <div class="w-full relative p-4" style="height: {data.totalSize}px;">
        <div
          class="absolute top-0 left-0 w-full px-4"
          style="transform: translateY({data.offsetY}px);"
        >
          <div class="grid gap-4 grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
            <VirtualizedOccurrenceList
              virtualItems={data.virtualItems}
              scrollToIndex={data.scrollToIndex}
              {occurrenceCache}
              {occurrenceCacheVersion}
              {coreIdColumn}
              {count}
            >
              {#snippet children({ virtualRow, occurrence, handleOccurrenceClick, selectedOccurrenceIndex })}
              <div class="flex" style="height: {virtualRow.size}px;">
                {#if occurrence}
                  <button
                    type="button"
                    class={{
                      "w-full text-left": true,
                      "outline-2 outline-primary-200": virtualRow.index === selectedOccurrenceIndex
                    }}
                    onclick={() => handleOccurrenceClick(occurrence, virtualRow.index)}
                  >
                    <OccurrenceCard {occurrence} />
                  </button>
                {:else}
                  <div class="loading-card w-full card preset-filled-surface-100-900 border-surface-200-800
                    rounded-md border-2 p-4 text-center text-gray-400"
                    style="height: {virtualRow.size}px;">
                    Loading...
                  </div>
                {/if}
              </div>
              {/snippet}
            </VirtualizedOccurrenceList>
          </div>
        </div>
      </div>
    {/key}
  {/snippet}
</VirtualizedList>
{/key}
