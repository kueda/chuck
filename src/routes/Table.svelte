<script lang="ts">
  import { ArrowUpIcon, ArrowDownIcon } from 'lucide-svelte';
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
    coreIdColumn: string;
    scrollState: { targetIndex: number; shouldScroll: boolean };
    currentSortColumn?: string;
    currentSortDirection?: string;
    onColumnHeaderClick: (column: string) => void;
  }

  const {
    occurrenceCache,
    occurrenceCacheVersion,
    count,
    scrollElement,
    onVisibleRangeChange,
    coreIdColumn,
    scrollState,
    currentSortColumn = '',
    currentSortDirection = '',
    onColumnHeaderClick,
  }: Props = $props();

  // Define visible columns with display names
  const columns = $derived([
    { field: coreIdColumn, label: coreIdColumn },
    { field: 'scientificName', label: 'scientificName' },
    { field: 'decimalLatitude', label: 'lat' },
    { field: 'decimalLongitude', label: 'lng' },
    { field: 'eventDate', label: 'eventDate' },
    { field: 'eventTime', label: 'eventTime' },
  ]);

  function getSortIcon(column: string) {
    if (currentSortColumn !== column) return null;
    return currentSortDirection === 'ASC' ? ArrowUpIcon : ArrowDownIcon;
  }
</script>

<VirtualizedList
  {count}
  {scrollElement}
  estimateSize={
    // Row height and border
    40 + 1
  }
  lanes={1}
  {onVisibleRangeChange}
  {scrollState}
>
  {#snippet children(data: VirtualListData)}
    {#key data._key}
      <div class="occurrence-table w-full">
        <div class="flex items-center py-2 px-2 border-b font-bold">
          {#each columns as column}
            <button
              class="table-header-cell flex text-left hover:bg-gray-100 cursor-pointer items-center gap-1 flex-row flex-nowrap"
              class:flex-1={column.field === coreIdColumn || column.field === 'scientificName'}
              class:w-24={column.field === 'decimalLatitude' || column.field === 'decimalLongitude'}
              class:w-32={column.field === 'eventDate' || column.field === 'eventTime'}
              onclick={() => onColumnHeaderClick(column.field)}
            >
              <span>{column.label}</span>
              {#if getSortIcon(column.field)}
                <svelte:component this={getSortIcon(column.field)} size={14} />
              {/if}
            </button>
          {/each}
        </div>
        <div class="w-full relative" style="height: {data.totalSize}px;">
          <div
            class="absolute top-0 left-0 w-full"
            style="transform: translateY({data.offsetY}px);"
          >
            <VirtualizedOccurrenceList
              virtualItems={data.virtualItems}
              scrollToIndex={data.scrollToIndex}
              {occurrenceCache}
              {occurrenceCacheVersion}
              {coreIdColumn}
              {count}
            >
              {#snippet children({ virtualRow, occurrence, handleOccurrenceClick, selectedOccurrenceIndex })}
              <div
                class={{
                  "occurrence-item occurrence-row flex items-center py-2 px-2 border-b cursor-pointer hover:bg-gray-100": true,
                  "outline-2 outline-primary-200": virtualRow.index === selectedOccurrenceIndex
                }}
                style="height: {virtualRow.size}px;"
                onclick={() => occurrence && handleOccurrenceClick(occurrence, virtualRow.index)}
                onkeydown={(e) => {
                  if (occurrence && (e.key === 'Enter' || e.key === ' ')) {
                    e.preventDefault();
                    handleOccurrenceClick(occurrence, virtualRow.index);
                  }
                }}
                role="button"
                tabindex="0"
              >
                {#if occurrence}
                  <div class="table-cell flex-1 truncate">{occurrence[coreIdColumn as keyof Occurrence]}</div>
                  <div class="table-cell flex-1 truncate">{occurrence.scientificName}</div>
                  <div class="table-cell w-24 truncate">{occurrence.decimalLatitude}</div>
                  <div class="table-cell w-24 truncate">{occurrence.decimalLongitude}</div>
                  <div class="table-cell w-32 truncate">{occurrence.eventDate}</div>
                  <div class="table-cell w-32 truncate">{occurrence.eventTime}</div>
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
