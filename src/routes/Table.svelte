<script lang="ts">
  import { ArrowUpIcon, ArrowDownIcon, Columns3Cog } from 'lucide-svelte';
  import { Popover, Portal, useListCollection } from '@skeletonlabs/skeleton-svelte';
  import VirtualizedList from '$lib/components/VirtualizedList.svelte';
  import VirtualizedOccurrenceList from '$lib/components/VirtualizedOccurrenceList.svelte';
  import type {
    VirtualListData,
    Props as VirtualizedListProps
  } from '$lib/components/VirtualizedList.svelte';
  import type { Occurrence } from '$lib/types/archive';
  import { getColumnWidthClass } from '$lib/utils/columnWidth';

  interface Props extends Pick<VirtualizedListProps, 'count' | 'scrollElement' | 'onVisibleRangeChange'> {
    occurrenceCache: Map<number, Occurrence>;
    occurrenceCacheVersion: number;
    coreIdColumn: string;
    archiveName: string;
    availableColumns: string[];
    visibleColumns: string[];
    scrollState: { targetIndex: number; shouldScroll: boolean };
    currentSortColumn?: string;
    currentSortDirection?: string;
    onColumnHeaderClick: (column: string) => void;
    onVisibleColumnsChange: (columns: string[]) => void;
  }

  const {
    occurrenceCache,
    occurrenceCacheVersion,
    count,
    scrollElement,
    onVisibleRangeChange,
    coreIdColumn,
    archiveName,
    availableColumns,
    visibleColumns,
    scrollState,
    currentSortColumn = '',
    currentSortDirection = '',
    onColumnHeaderClick,
    onVisibleColumnsChange,
  }: Props = $props();

  // Map field names to display labels
  function getColumnLabel(field: string): string {
    if (field === 'decimalLatitude') return 'lat';
    if (field === 'decimalLongitude') return 'lng';
    return field;
  }

  // Define visible columns with display names
  const columns = $derived(
    visibleColumns.map(field => ({ field, label: getColumnLabel(field) }))
  );

  let searchText = $state('');

  // Filtered columns for the listbox
  const filteredColumns = $derived(
    availableColumns.filter(col =>
      col.toLowerCase().includes(searchText.toLowerCase())
    )
  );

  // Create listbox collection
  const collection = $derived(
    useListCollection({
      items: filteredColumns.map(col => ({ value: col, label: col })),
      itemToString: (item) => item.label,
      itemToValue: (item) => item.value
    })
  );

  function toggleColumn(column: string) {
    const newColumns = visibleColumns.includes(column)
      ? visibleColumns.filter(c => c !== column)
      : [...visibleColumns, column];

    // Ensure at least one column is selected
    if (newColumns.length === 0) {
      return;
    }

    onVisibleColumnsChange(newColumns);
  }

  function isColumnVisible(column: string): boolean {
    return visibleColumns.includes(column);
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
      <div class="occurrence-table w-full overflow-x-auto">
        <div class="flex items-center py-2 px-2 border-b font-bold min-w-max">
          <div class="table-header-cell flex flex-row w-8 shrink-0">
            <Popover>
              <Popover.Trigger>
                <button type="button" class="hover:bg-gray-100 p-1 rounded">
                  <Columns3Cog size={16} />
                </button>
              </Popover.Trigger>
              <Portal>
                <Popover.Positioner>
                  <Popover.Content class="card p-4 bg-surface-100-900 shadow-lg max-h-96 overflow-y-auto w-64">
                    <div class="mb-2">
                      <input
                        type="text"
                        placeholder="Search columns..."
                        class="input w-full"
                        autocapitalize="off"
                        autocorrect="off"
                        bind:value={searchText}
                      />
                    </div>
                    <div class="space-y-1">
                      {#each filteredColumns as column}
                        <label class="flex items-center gap-2 hover:bg-gray-100 p-2 rounded cursor-pointer">
                          <input
                            type="checkbox"
                            checked={isColumnVisible(column)}
                            onchange={() => toggleColumn(column)}
                            disabled={visibleColumns.length === 1 && isColumnVisible(column)}
                            class="checkbox"
                          />
                          <span class="text-sm">{column}</span>
                        </label>
                      {/each}
                    </div>
                  </Popover.Content>
                </Popover.Positioner>
              </Portal>
            </Popover>
          </div>
          {#each columns as column}
            <button
              class="table-header-cell text-left hover:bg-gray-100 cursor-pointer items-center gap-1 flex flex-row flex-nowrap shrink-0 {getColumnWidthClass(column.field)} p-1"
              onclick={() => onColumnHeaderClick(column.field)}
            >
              <span class="truncate">{column.label}</span>
              {#if currentSortColumn === column.field}
                {#if currentSortDirection === 'ASC'}
                  <ArrowUpIcon size={14} />
                {:else}
                  <ArrowDownIcon size={14} />
                {/if}
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
                  "occurrence-item occurrence-row flex items-center py-2 px-2 border-b cursor-pointer hover:bg-gray-100 min-w-max": true,
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
                  <div class="table-cell w-8 shrink-0"></div>
                  {#each columns as column}
                    <div class="table-cell truncate shrink-0 {getColumnWidthClass(column.field)} p-1">
                      {occurrence[column.field as keyof Occurrence]}
                    </div>
                  {/each}
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
