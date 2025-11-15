<script lang="ts">
  import { Accordion } from '@skeletonlabs/skeleton-svelte';
  import ComboboxFilter from './ComboboxFilter.svelte';
  import { categorizeColumns } from '$lib/utils/filterCategories';
  import type { SearchParams } from '$lib/utils/filterCategories';
  import { MinusIcon, PlusIcon } from 'lucide-svelte';

  const NUMERIC_COLUMNS = [
    'decimalLatitude',
    'decimalLongitude',
    'gbifID',
    'nelat',
    'nelng',
    'swlat',
    'swlng',
  ];

  interface Props {
    onSearchChange: (params: SearchParams) => void;
    availableColumns?: (keyof SearchParams)[];
    initialSortBy?: string;
    initialSortDirection?: 'ASC' | 'DESC';
    searchParams?: SearchParams;
  }

  let { onSearchChange, availableColumns = [], searchParams }: Props = $props();

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
    if (searchParams?.order_by !== lastInitialSortBy) {
      sortBy = searchParams?.order_by || '';
      lastInitialSortBy = searchParams?.order_by;
    }
    if (searchParams?.order !== lastInitialSortDirection) {
      sortDirection = searchParams?.order || '';
      lastInitialSortDirection = searchParams?.order;
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
    filterCategories.map(category => ({
      category,
      activeCount: category.columns.filter(c => localParams[c]).length,
      hasActive: category.columns.some(c => localParams[c])
    }))
  );

  function triggerSearch() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    debounceTimer = setTimeout(() => {
      const params: SearchParams = {};

      // Add filters directly at the top level (flattened structure)
      for (const [key, value] of Object.entries(localParams)) {
        if (value && value.trim()) {
          // Type assertion needed because TypeScript can't verify dynamic keys match the interface
          (params as Record<string, string>)[key] = value;
        }
      }

      // Add sorting
      if (sortBy) {
        params.order_by = sortBy;
        if (sortDirection) {
          params.order = sortDirection;
        }
      }

      onSearchChange(params);
    }, 300);
  }

  function handleFilterChange(columnName: keyof SearchParams, value: any) {
    // Don't trigger search if we're syncing from prop
    if (syncingFromProp) return;

    if (value) {
      localParams[columnName] = value;
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
      <Accordion.ItemTrigger class="flex justify-between items-center p-0 hover:bg-transparent">
        Sort
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
        <Accordion.ItemTrigger class="flex justify-between items-center p-2 hover:bg-transparent">
          {#if hasActive}
            <span>
              {category.name} (<span class="font-bold">{activeCount}</span> / {category.columns.length})
            </span>
          {:else}
            <span>
              {category.name} ({category.columns.length})
            </span>
          {/if}
          <Accordion.ItemIndicator class="group">
            <MinusIcon class="size-4 group-data-[state=open]:block hidden" />
            <PlusIcon class="size-4 group-data-[state=open]:hidden block" />
          </Accordion.ItemIndicator>
        </Accordion.ItemTrigger>
        <Accordion.ItemContent class="p-0">
          {#each category.columns as columnName (columnName)}
            <ComboboxFilter
              {columnName}
              value={localParams[columnName] ? String(localParams[columnName]) : ''}
              onValueChange={(value) => handleFilterChange(columnName, value)}
              onClear={() => handleFilterClear(columnName)}
              type={NUMERIC_COLUMNS.includes(columnName) ? 'number' : 'text'}
            />
          {/each}
        </Accordion.ItemContent>
      </Accordion.Item>
    {/each}
  </Accordion>
</div>
