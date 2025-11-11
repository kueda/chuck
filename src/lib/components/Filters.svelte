<script lang="ts">
  import { Accordion } from '@skeletonlabs/skeleton-svelte';
  import ComboboxFilter from './ComboboxFilter.svelte';
  import { categorizeColumns } from '$lib/utils/filterCategories';
  import { MinusIcon, PlusIcon } from 'lucide-svelte';

  const NUMERIC_COLUMNS = [
    'decimalLatitude',
    'decimalLongitude',
    'gbifID',
  ];

  // SearchParams matches the backend's flattened structure (using #[serde(flatten)])
  // All Darwin Core fields are at the same level as order_by/order
  // Note: The Darwin Core "order" taxonomy field conflicts with the sort "order" field,
  // so it cannot be filtered (backend reserves "order" and "order_by")
  export interface SearchParams {
    // Taxonomy
    scientificName?: string;
    genus?: string;
    family?: string;
    // order?: string;  // CONFLICT: reserved for sort direction
    class?: string;
    phylum?: string;
    kingdom?: string;
    taxonRank?: string;
    taxonomicStatus?: string;
    vernacularName?: string;
    specificEpithet?: string;
    infraspecificEpithet?: string;
    taxonID?: string;
    higherClassification?: string;
    subfamily?: string;
    tribe?: string;
    subtribe?: string;
    superfamily?: string;
    subgenus?: string;
    genericName?: string;
    infragenericEpithet?: string;

    // Geography
    locality?: string;
    stateProvince?: string;
    county?: string;
    municipality?: string;
    country?: string;
    countryCode?: string;
    continent?: string;
    waterBody?: string;
    island?: string;
    islandGroup?: string;
    higherGeography?: string;
    verbatimLocality?: string;

    // Temporal
    eventDate?: string;
    eventTime?: string;
    year?: string;
    month?: string;
    day?: string;
    dateIdentified?: string;
    modified?: string;
    created?: string;
    verbatimEventDate?: string;

    // Identification
    recordedBy?: string;
    identifiedBy?: string;
    recordNumber?: string;
    catalogNumber?: string;
    otherCatalogNumbers?: string;
    fieldNumber?: string;
    occurrenceID?: string;
    institutionCode?: string;
    collectionCode?: string;

    // Sorting (reserved field names, not filters)
    order_by?: string;
    order?: 'ASC' | 'DESC';
  }

  interface Props {
    onSearchChange: (params: SearchParams) => void;
    availableColumns?: string[];
    initialSortBy?: string;
    initialSortDirection?: 'ASC' | 'DESC';
  }

  let { onSearchChange, availableColumns = [], initialSortBy, initialSortDirection }: Props = $props();

  let activeFilters = $state<Record<string, string>>({});
  let sortBy = $state<string>('');
  let sortDirection = $state<'ASC' | 'DESC' | ''>('');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Track the last prop values to detect when they change
  let lastInitialSortBy = $state<string | undefined>(undefined);
  let lastInitialSortDirection = $state<'ASC' | 'DESC' | undefined>(undefined);

  // Sync state with initial props when they change
  $effect(() => {
    // Only update if the PROPS changed (not internal state)
    if (initialSortBy !== lastInitialSortBy) {
      sortBy = initialSortBy || '';
      lastInitialSortBy = initialSortBy;
    }
    if (initialSortDirection !== lastInitialSortDirection) {
      sortDirection = initialSortDirection || '';
      lastInitialSortDirection = initialSortDirection;
    }
  });

  // Categorize columns for accordion sections
  const filterCategories = $derived(categorizeColumns(availableColumns));

  function triggerSearch() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    debounceTimer = setTimeout(() => {
      const params: SearchParams = {};

      // Add filters directly at the top level (flattened structure)
      for (const [key, value] of Object.entries(activeFilters)) {
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

  function handleFilterChange(columnName: string, value: string) {
    if (value) {
      activeFilters[columnName] = value;
    } else {
      delete activeFilters[columnName];
    }
    activeFilters = { ...activeFilters }; // Trigger reactivity
    triggerSearch();
  }

  function handleFilterClear(columnName: string) {
    delete activeFilters[columnName];
    activeFilters = { ...activeFilters }; // Trigger reactivity
    triggerSearch();
  }

  function handleSortByChange() {
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
              <select class="select" bind:value={sortDirection} onchange={triggerSearch}>
                <option value="ASC">ASC</option>
                <option value="DESC">DESC</option>
              </select>
            </label>
          </div>
        {/if}
      </Accordion.ItemContent>
    </Accordion.Item>
    {#each filterCategories as category (category.name)}
      <Accordion.Item
        value={category.name}
        class="p-0 [&[data-state=open]]:bg-gray-100 hover:bg-gray-100"
      >
        <Accordion.ItemTrigger class="flex justify-between items-center p-2 hover:bg-transparent">
          <span>
            {category.name}
            {#if category.columns.find(c => activeFilters[c])}
              (<span class="font-bold">{category.columns.filter(c => activeFilters[c]).length}</span> / {category.columns.length})
            {:else}
              ({category.columns.length})
            {/if}
          </span>
          <Accordion.ItemIndicator class="group">
            <MinusIcon class="size-4 group-data-[state=open]:block hidden" />
            <PlusIcon class="size-4 group-data-[state=open]:hidden block" />
          </Accordion.ItemIndicator>
        </Accordion.ItemTrigger>
        <Accordion.ItemContent class="p-0">
          {#each category.columns as columnName (columnName)}
            <ComboboxFilter
              {columnName}
              value={activeFilters[columnName] || ''}
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
