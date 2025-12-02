<script lang="ts">
  import { onMount } from 'svelte';
  import VirtualizedList from '$lib/components/VirtualizedList.svelte';
  import VirtualizedOccurrenceList from '$lib/components/VirtualizedOccurrenceList.svelte';
  import OccurrenceDrawer from '$lib/components/OccurrenceDrawer.svelte';
  import { createDrawerHandlers, type DrawerState } from '$lib/utils/drawerState';
  import type {
    VirtualListData,
    Props as VirtualizedListProps
  } from '$lib/components/VirtualizedList.svelte';
  import type { Occurrence } from '$lib/types/archive';
  import OccurrenceCard, { EST_HEIGHT } from '$lib/components/OccurrenceCard.svelte';

  interface Props extends Pick<VirtualizedListProps, 'count' | 'scrollElement' | 'onVisibleRangeChange'> {
    drawerState: DrawerState;
    occurrenceCache: Map<number, Occurrence>;
    occurrenceCacheVersion: number;
    coreIdColumn: string;
    scrollState: { targetIndex: number; shouldScroll: boolean };
  }

  let {
    drawerState,
    occurrenceCache,
    occurrenceCacheVersion,
    count,
    scrollElement,
    onVisibleRangeChange,
    coreIdColumn,
    scrollState
  }: Props = $props();

  // Local scrollToIndex reference (still needed for virtualizer integration)
  let scrollToIndex = $state<((index: number, options?: { align?: 'start' | 'center' | 'end' | 'auto' }) => void) | undefined>();

  // Create drawer handlers
  const drawerHandlers = $derived(createDrawerHandlers({
    state: drawerState,
    occurrenceCache,
    coreIdColumn,
    count,
    scrollToIndex
  }));

  // Tanstack Virtual needs to know whether the virtualized items are arranged
  // in columns (which it calls lanes) and if so how many, so we're
  // recreating tailwind's breakpoints here to ensure the virtualizer knows
  // how many columns there are and that it gets recreated when the number of
  // columns change
  //
  // Note: changing the default here to window.innerWidth seems reasonable,
  // but seriously impacts performance for large result sets
  let windowWidth = $state(0);
  const lanes = $derived.by(() => {
    if (windowWidth >= 1536) return 6;
    if (windowWidth >= 1280) return 5;
    if (windowWidth >= 1024) return 4;
    return 3;
  });

  // Track window width so we can responsively tell the virtualizer how many
  // columns there are, which allows it to calculate heights correctly
  onMount(() => {
    const updateWidth = () => {
      // Ideally this guard prevents unnecessary updates to windowWidth, the
      // derived lanes value, and therefore unnecessary recreations of the
      // virtualizer
      if (windowWidth !== window.innerWidth) {
        windowWidth = window.innerWidth;
      }
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
  Force VirtualizedList to remount when count changes.
  This solves a Svelte 5 reactivity issue where TanStack Virtual's virtualizer
  doesn't properly recreate when count changes rapidly (e.g., 1000 → 0 → 5 during search).
  Without this, the virtualizer renders virtual items based on stale count values,
  showing "Loading..." placeholders for items that don't exist.

  Note: We only key on count, not occurrenceCacheVersion, to avoid remounting when
  chunks load during normal scrolling. This prevents scroll position resets.
-->
{#key count}
<VirtualizedList
  {count}
  {scrollElement}
  estimateSize={
    // Height of the card + gap
    EST_HEIGHT + 16
  }
  {lanes}
  {onVisibleRangeChange}
  {scrollState}
>
  {#snippet children(data: VirtualListData)}
    {#if data.scrollToIndex}
      {(scrollToIndex = data.scrollToIndex, '')}
    {/if}
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
              {occurrenceCache}
              {occurrenceCacheVersion}
              {coreIdColumn}
              handleOccurrenceClick={drawerHandlers.handleOccurrenceClick}
              selectedOccurrenceIndex={drawerState.selectedOccurrenceIndex}
            >
              {#snippet children({ virtualRow, occurrence })}
              <div class="flex" style="height: {virtualRow.size}px;">
                {#if occurrence}
                  <button
                    type="button"
                    class={{
                      "w-full text-left": true,
                      "outline-2 outline-primary-200": virtualRow.index === drawerState.selectedOccurrenceIndex
                    }}
                    onclick={() => drawerHandlers.handleOccurrenceClick(occurrence, virtualRow.index)}
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

<OccurrenceDrawer
  bind:open={drawerState.open}
  occurrenceId={drawerState.selectedOccurrenceId}
  {coreIdColumn}
  onClose={drawerHandlers.handleClose}
  onPrevious={drawerHandlers.handlePrevious}
  onNext={drawerHandlers.handleNext}
/>
