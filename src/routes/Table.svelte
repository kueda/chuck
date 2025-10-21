<script lang="ts">
  import VirtualizedList from '$lib/components/VirtualizedList.svelte';
  import VirtualizedOccurrenceList from '$lib/components/VirtualizedOccurrenceList.svelte';
  import type {
    VirtualListData,
    Props as VirtualizedListProps
  } from '$lib/components/VirtualizedList.svelte';
  import type { Occurrence } from '$lib/types/archive';

  interface Props extends Pick<VirtualizedListProps, 'count' | 'scrollElement' | 'onVisibleRangeChange'> {
    occurrenceCache: Map<number, Occurrence>;
    occurrenceCacheVersion: number;
  }

  const {
    occurrenceCache,
    occurrenceCacheVersion,
    count,
    scrollElement,
    onVisibleRangeChange
  }: Props = $props();
</script>

<VirtualizedList
  {count}
  {scrollElement}
  estimateSize={40}
  lanes={1}
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
      <div class="w-full">
        <div class="flex items-center py-2 px-2 border-b font-bold">
          <div class="flex-1">occurrenceID</div>
          <div class="flex-1">scientificName</div>
          <div class="w-24">lat</div>
          <div class="w-24">lng</div>
          <div class="w-32">eventDate</div>
          <div class="w-32">eventTime</div>
        </div>
        <div class="w-full relative" style="height: {data.totalSize}px;">
          <div
            class="absolute top-0 left-0 w-full"
            style="transform: translateY({data.offsetY}px);"
          >
            <VirtualizedOccurrenceList
              virtualItems={data.virtualItems}
              {occurrenceCache}
              {occurrenceCacheVersion}
            >
              {#snippet children({ virtualRow, occurrence })}
              <div
                class="flex items-center py-2 px-2 border-b"
                style="height: {virtualRow.size}px;"
              >
                {#if occurrence}
                  <div class="flex-1 truncate">{occurrence.occurrenceID}</div>
                  <div class="flex-1 truncate">{occurrence.scientificName}</div>
                  <div class="w-24 truncate">{occurrence.decimalLatitude}</div>
                  <div class="w-24 truncate">{occurrence.decimalLongitude}</div>
                  <div class="w-32 truncate">{occurrence.eventDate}</div>
                  <div class="w-32 truncate">{occurrence.eventTime}</div>
                {:else}
                  <div class="flex-1 text-gray-400">Loading...</div>
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
