<!--
  Reasonable defaults for a virtualized list of anything, with a little UI for
  loading and empty states.
-->

<script lang="ts">
  import { createVirtualizer, type Virtualizer, type VirtualItem } from '@tanstack/svelte-virtual';
  import { untrack } from 'svelte';
  import { get } from 'svelte/store';
  import type { Snippet } from 'svelte';

  export interface VirtualListData {
    virtualItems: VirtualItem[];
    totalSize: number;
    offsetY: number;
    scrollToIndex?: (index: number, options?: { align?: 'start' | 'center' | 'end' | 'auto' }) => void;
    _key?: number;
  }

  export interface Props {
    count: number;
    scrollElement: Element | undefined;
    estimateSize: number;
    lanes: number;
    onVisibleRangeChange?: (
      range: {
        firstIndex: number,
        lastIndex: number,
        scrollOffsetIndex: number | undefined
      }
    ) => void;
    scrollState?: { targetIndex: number; shouldScroll: boolean };
    children: Snippet<[VirtualListData]>;
  }

  let {
    count,
    scrollElement,
    estimateSize,
    lanes,
    onVisibleRangeChange,
    scrollState,
    children
  }: Props = $props();

  // Note: virtualizer is a store from TanStack Virtual, not wrapped in $state
  // to avoid infinite loops. Use initialized flag for initial render trigger.
  // Using type assertion (as) instead of type annotation (:) because when Svelte's
  // template compiler sees $virtualizer, it needs to unwrap the store type. With a
  // strict type annotation, TypeScript's inference fails and resolves to 'never'.
  // Type assertion allows TypeScript to infer the unwrapped type correctly.
  //
  // svelte-ignore non_reactive_update
  let virtualizer = null as ReturnType<typeof createVirtualizer<Element, Element>> | null;

  // We set up the virtualizer once at startup in an effect which we never
  // want to run again
  let initialized = $state(false);

  // Due to mysterious mysteries, recreating the virtualizer doesn't always
  // result in the heights set by virtualRow.size getting reset when the
  // virtualized items render which causes problems when toggling between
  // views where items have different heights... even though the virtualizer
  // gets trashed and recreated and so does the DOM. I don't understand why,
  // but using this key to wrap the relevant parts of the DOM fixes it.
  let virtualizerVersion = $state(0);

  // Performance optimization: cache scrollOffsetIndex and only recalculate
  // when scroll position changes significantly, i.e. more than the height of
  // a virtualized item
  let lastScrollOffsetIndex: number | undefined = undefined;
  let lastCalculatedScrollTop = 0;
  let lastReportedFirstIndex = -1;
  let lastReportedLastIndex = -1;

  function handleChange(instance: Virtualizer<Element, Element>) {
    if (!onVisibleRangeChange) return;

    const items = instance.getVirtualItems();
    if (items.length === 0) return;

    const firstIndex = items[0].index;
    const lastIndex = items[items.length - 1].index;

    // Only calculate scrollOffsetIndex if scroll position changed significantly
    // Skip calculation entirely if at scroll position 0 (initial render)
    let scrollOffsetIndex = lastScrollOffsetIndex;
    const currentScrollTop = instance.scrollOffset || 0;

    if (
      // Don't need to preserve scroll position if we're at the top
      currentScrollTop > 0
      // Don't need to preserve scroll position if we haven't scrolled more
      // than the height of an item
      && Math.abs(currentScrollTop - lastCalculatedScrollTop) > estimateSize
    ) {
      const rowHeight = items[0].size;
      const scrollOffsetRows = Math.floor(currentScrollTop / rowHeight);
      scrollOffsetIndex = scrollOffsetRows * lanes;
      lastScrollOffsetIndex = scrollOffsetIndex;
      lastCalculatedScrollTop = currentScrollTop;
    }

    // Only call callback if range actually changed
    if (
      firstIndex !== lastReportedFirstIndex
      || lastIndex !== lastReportedLastIndex
    ) {
      lastReportedFirstIndex = firstIndex;
      lastReportedLastIndex = lastIndex;
      onVisibleRangeChange({ firstIndex, lastIndex, scrollOffsetIndex });
    }
  }

  // Wrapper for the Tanstack Virtualizer function so we can pass it down to children
  function scrollToIndex(index: number, options?: { align?: 'start' | 'center' | 'end' | 'auto' }) {
    if (!virtualizer) return;
    const instance = get(virtualizer);
    if (instance?.scrollToIndex) {
      instance.scrollToIndex(index, options);
    }
  }

  // Initialize virtualizer when dependencies change
  // Recreate the virtualizer when count, lanes, or estimateSize change
  $effect(() => {
    if (!scrollElement) return;

    // Capture current scroll position before recreating virtualizer
    const currentScrollOffset = scrollElement.scrollTop;

    virtualizer = createVirtualizer({
      count,
      getScrollElement: () => scrollElement ?? null,
      estimateSize: () => estimateSize,
      overscan: 50,
      lanes,
      onChange: handleChange,
      // Preserve scroll position when virtualizer is recreated
      initialOffset: currentScrollOffset
    });

    untrack(() => {
      virtualizerVersion++;
    });
    initialized = true;
  });

  // Separate effect to handle scroll to index when view switches
  $effect(() => {
    if (!scrollState) return;
    if (!virtualizer || !initialized) return;
    if (!scrollState.shouldScroll || scrollState.targetIndex === 0) return;

    const { targetIndex } = scrollState;
    // Scroll to the target index after a delay
    setTimeout(() => {
      if (virtualizer && scrollState.shouldScroll) {
        scrollToIndex(targetIndex, { align: 'start' });
        // Mark as consumed
        scrollState.shouldScroll = false;
      }
    }, 100);
  });
</script>

{#if initialized && virtualizer}
  {@const items = $virtualizer!.getVirtualItems()}
  {@const rawTotalSize = $virtualizer!.getTotalSize()}
  {@const offsetY = items[0]?.start ?? 0}
  <!--
    Provide a reasonable cap on the height of the scrollable; rendering
    insanely tall elements creates lag. This isn't a huge boost, so jettison
    if it's causing problems.
  -->
  {@const totalSize = Math.min(rawTotalSize, 33_554_428)}
  {@const virtualData = {
    virtualItems: items,
    totalSize: totalSize,
    offsetY: offsetY,
    scrollToIndex: scrollToIndex,
    _key: virtualizerVersion
  }}

  {#if count === 0}
    <div class="w-full h-full flex justify-center items-center">
      <p>No results</p>
    </div>
  {:else}
    {@render children(virtualData)}
  {/if}
{:else}
  <div class="p-4">Loading...</div>
{/if}
