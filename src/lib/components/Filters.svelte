<script lang="ts">
import { Accordion } from '@skeletonlabs/skeleton-svelte';
import { ArrowUpDown, MinusIcon, PlusIcon } from 'lucide-svelte';
import type { SearchParams } from '$lib/utils/filterCategories';
import { categorizeColumns } from '$lib/utils/filterCategories';
import ComboboxFilter from './ComboboxFilter.svelte';
import MinMaxFilter from './MinMaxFilter.svelte';

const NUMERIC_COLUMNS = [
  'decimalLatitude',
  'decimalLongitude',
  'gbifID',
  'nelat',
  'nelng',
  'swlat',
  'swlng',
];

const MIN_MAX_COLUMNS = ['coordinateUncertaintyInMeters', 'elevation'];

// Helper to access localParams with dynamic keys (e.g. `${col}_min`)
function param(key: string): string {
  return (localParams as Record<string, string>)[key] ?? '';
}

interface Props {
  onSearchChange: (params: SearchParams) => void;
  availableColumns?: (keyof SearchParams)[];
  initialSortBy?: string;
  initialSortDirection?: 'ASC' | 'DESC';
  searchParams?: SearchParams;
}

const { onSearchChange, availableColumns = [], searchParams }: Props = $props();

// Track local state separately to manage things like debounce
let localParams = $state<SearchParams>({});
let sortBy = $state<string>('');
let sortDirection = $state<'ASC' | 'DESC' | ''>('');
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let syncingFromProp = $state(false);

// Track the last prop values to detect when they change
let lastInitialSortBy = $state<string | undefined>(undefined);
let lastInitialSortDirection = $state<'ASC' | 'DESC' | undefined>(undefined);

// Sync state with initial props when they change
$effect(() => {
  // Only update if the PROPS changed (not internal state)
  syncingFromProp = true;
  if (searchParams?.sort_by !== lastInitialSortBy) {
    sortBy = searchParams?.sort_by || '';
    lastInitialSortBy = searchParams?.sort_by;
  }
  if (searchParams?.sort_direction !== lastInitialSortDirection) {
    sortDirection = searchParams?.sort_direction || '';
    lastInitialSortDirection = searchParams?.sort_direction;
  }
  syncingFromProp = false;
});

// Sync localParams from searchParam prop when it changes
$effect(() => {
  if (!searchParams) return;

  syncingFromProp = true;
  const newParams: SearchParams = {};
  for (const [key, value] of Object.entries(searchParams)) {
    if (value) newParams[key as keyof SearchParams] = value;
  }
  localParams = searchParams;

  // Keep syncingFromProp true for a moment to prevent any immediate reactions
  setTimeout(() => {
    syncingFromProp = false;
  }, 0);
});

// Categorize columns for accordion sections
const filterCategories = $derived(categorizeColumns(availableColumns));

// Track active params counts per category (reactive to localParams changes)
const categoryCounts = $derived.by(() =>
  filterCategories.map((category) => {
    const activeCount = category.columns.filter((c) => {
      if (MIN_MAX_COLUMNS.includes(c)) {
        return param(`${c}_min`) || param(`${c}_max`);
      }
      return localParams[c];
    }).length;
    return {
      category,
      activeCount,
      hasActive: activeCount > 0,
    };
  }),
);

function triggerSearch() {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }

  debounceTimer = setTimeout(() => {
    const params: SearchParams = {};

    // Add filters directly at the top level (flattened structure)
    for (const [key, value] of Object.entries(localParams)) {
      if (value?.trim()) {
        // Type assertion needed because TypeScript can't verify dynamic keys match the interface
        (params as Record<string, string>)[key] = value;
      }
    }

    // Add sorting
    if (sortBy) {
      params.sort_by = sortBy;
      if (sortDirection) {
        params.sort_direction = sortDirection;
      }
    }

    onSearchChange(params);
  }, 300);
}

function handleFilterChange(columnName: keyof SearchParams, value: string) {
  // Don't trigger search if we're syncing from prop
  if (syncingFromProp) return;

  if (value) {
    (localParams as Record<string, string>)[columnName] = value;
  } else {
    delete localParams[columnName];
  }
  localParams = { ...localParams }; // Trigger reactivity
  triggerSearch();
}

function handleFilterClear(columnName: keyof SearchParams) {
  // Don't trigger search if we're syncing from prop
  if (syncingFromProp) return;

  delete localParams[columnName];
  localParams = { ...localParams }; // Trigger reactivity
  triggerSearch();
}

function handleMinMaxChange(
  columnName: keyof SearchParams,
  suffix: '_min' | '_max' | '_include_blank',
  value: string | boolean,
) {
  if (syncingFromProp) return;
  const key = `${columnName}${suffix}` as keyof SearchParams;
  const strValue = String(value);
  if (strValue && strValue !== 'false') {
    (localParams as Record<string, string>)[key] = strValue;
  } else {
    delete localParams[key];
  }
  localParams = { ...localParams };
  triggerSearch();
}

function handleSortByChange() {
  // Don't trigger search if we're syncing from prop
  if (syncingFromProp) return;

  // When column changes, default to ASC if a column is selected
  if (sortBy && !sortDirection) {
    sortDirection = 'ASC';
  }
  triggerSearch();
}
</script>

<div id="Filters" class="mb-4">
  <Accordion multiple collapsible>
    <Accordion.Item value="sort" class="[&[data-state=open]]:bg-gray-100 p-2 hover:bg-gray-100">
      <Accordion.ItemTrigger class="flex justify-between items-center p-0 hover:bg-transparent gap-2">
        <ArrowUpDown size={18} />
        <span class="flex justify-start grow-2">Sort</span>
        <Accordion.ItemIndicator class="group">
          <MinusIcon class="size-4 group-data-[state=open]:block hidden" />
          <PlusIcon class="size-4 group-data-[state=open]:hidden block" />
        </Accordion.ItemIndicator>
      </Accordion.ItemTrigger>
      <Accordion.ItemContent class="p-0">
        <!-- Sort Controls -->
        <div class="mb-4">
          <label for="sortBy" class="label">
            <span class="label-text">Sort By</span>
            <select
              id="sortBy"
              class="select"
              bind:value={sortBy}
              onchange={handleSortByChange}
            >
              <option value="">None</option>
              {#each availableColumns as column}
                <option value={column}>{column}</option>
              {/each}
            </select>
          </label>
        </div>

        {#if sortBy}
          <div class="mb-4">
            <label class="label">
              <span class="label-text">Sort Direction</span>
              <select class="select" bind:value={sortDirection} onchange={() => { if (!syncingFromProp) triggerSearch(); }}>
                <option value="ASC">ASC</option>
                <option value="DESC">DESC</option>
              </select>
            </label>
          </div>
        {/if}
      </Accordion.ItemContent>
    </Accordion.Item>
    {#each categoryCounts as { category, activeCount, hasActive } (category.name)}
      <Accordion.Item
        value={category.name}
        class="p-0 [&[data-state=open]]:bg-gray-100 hover:bg-gray-100"
      >
        <Accordion.ItemTrigger class="flex justify-between items-center p-2 hover:bg-transparent gap-2">
          {#if category.icon}
            <!-- <svelte:component this={category.icon} size={18} /> -->
            {@const CategoryIconComponent = category.icon}
            <CategoryIconComponent size={18}></CategoryIconComponent>
          {/if}
          <span class="flex justify-start grow-2">
            {#if hasActive}
              {category.name} (<span class="font-bold">{activeCount}</span> / {category.columns.length})
            {:else}
              {category.name} ({category.columns.length})
            {/if}
          </span>
          <Accordion.ItemIndicator class="group">
            <MinusIcon class="size-4 group-data-[state=open]:block hidden" />
            <PlusIcon class="size-4 group-data-[state=open]:hidden block" />
          </Accordion.ItemIndicator>
        </Accordion.ItemTrigger>
        <Accordion.ItemContent class="p-0">
          {#each category.columns as columnName (columnName)}
            {#if MIN_MAX_COLUMNS.includes(columnName)}
              <MinMaxFilter
                {columnName}
                minValue={param(`${columnName}_min`)}
                maxValue={param(`${columnName}_max`)}
                includeBlank={param(
                  `${columnName}_include_blank`,
                ) === 'true'}
                onMinChange={(v) =>
                  handleMinMaxChange(columnName, '_min', v)}
                onMaxChange={(v) =>
                  handleMinMaxChange(columnName, '_max', v)}
                onIncludeBlankChange={(v) =>
                  handleMinMaxChange(
                    columnName,
                    '_include_blank',
                    v,
                  )}
              />
            {:else}
              <ComboboxFilter
                {columnName}
                value={localParams[columnName]
                  ? String(localParams[columnName])
                  : ''}
                onValueChange={(value) =>
                  handleFilterChange(columnName, value)}
                onClear={() => handleFilterClear(columnName)}
                type={NUMERIC_COLUMNS.includes(columnName)
                  ? 'number'
                  : 'text'}
              />
            {/if}
          {/each}
        </Accordion.ItemContent>
      </Accordion.Item>
    {/each}
  </Accordion>
</div>
