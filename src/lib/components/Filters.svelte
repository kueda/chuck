<script lang="ts">
  import { ArrowUpIcon, ArrowDownIcon, XIcon } from 'lucide-svelte';

  export interface SearchParams {
    scientific_name?: string;
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

  let scientificName = $state<string>('');
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

  function triggerSearch() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    debounceTimer = setTimeout(() => {
      const start = performance.now();
      const params: SearchParams = {};
      if (scientificName.trim()) {
        params.scientific_name = scientificName.trim();
      }
      if (sortBy) {
        params.order_by = sortBy;
        if (sortDirection) {
          params.order = sortDirection;
        }
      }
      onSearchChange(params);
    }, 300);
  }

  function handleSortByChange() {
    // When column changes, default to ASC if a column is selected
    if (sortBy && !sortDirection) {
      sortDirection = 'ASC';
    }
    triggerSearch();
  }
</script>

<aside class="p-4 w-64">
  <h2 class="text-lg font-bold mb-4">Filters</h2>

  <div class="mb-4">
    <label for="scientificName" class="label">
      <span class="label-text">Scientific Name</span>
      <input
        id="scientificName"
        type="text"
        class="input w-full"
        bind:value={scientificName}
        oninput={triggerSearch}
        placeholder="Search..."
      />
    </label>
  </div>

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
</aside>
