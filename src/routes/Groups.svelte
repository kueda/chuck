<script lang="ts">
  import GroupRow from '$lib/components/GroupRow.svelte';
  import MediaItem from '$lib/components/MediaItem.svelte';
  import OccurrenceDrawer from '$lib/components/OccurrenceDrawer.svelte';
  import type { Occurrence } from '$lib/types/archive';
  import type { SearchParams } from '$lib/utils/filterCategories';
  import ViewSwitcher from '$lib/components/ViewSwitcher.svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    coreIdColumn: string;
    defaultSelectedField?: string;
    searchParams: SearchParams;
    varcharFields: string[];
    onCountClick: (fieldName: string, fieldValue: string | null) => void;
  }

  let {
    coreIdColumn,
    defaultSelectedField,
    searchParams,
    varcharFields,
    onCountClick
  }: Props = $props();

  interface AggregationResult {
    count: number;
    photoUrl?: string | null;
    value: string | null;
  }

  const AGGREGATION_LIMIT = 1000;

  let selectedField = $state(defaultSelectedField)
  let results = $state<AggregationResult[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let currentView = $state<'table' | 'cards' | 'rows'>('table');

  let drawerOpen = $state(false);
  let selectedOccurrenceId = $state<string | number | null>(null);

  async function fetchAggregation() {
    if (!selectedField) return;

    loading = true;
    error = null;

    try {
      const data = await invoke<AggregationResult[]>('aggregate_by_field', {
        fieldName: selectedField,
        searchParams,
        limit: AGGREGATION_LIMIT
      });
      results = data;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      results = [];
    } finally {
      loading = false;
    }
  }

  function handleCountClick(value: string | null) {
    if (!selectedField) return;
    onCountClick(selectedField, value);
  }

  // Automatically fetch when selectedField, searchParams, or currentView change
  $effect(() => {
    if (selectedField) {
      fetchAggregation();
    }
  });

  $effect(() => {
    if (defaultSelectedField && !selectedField) selectedField = defaultSelectedField;
  });
</script>

<div class="flex h-full flex-col gap-4 p-4">
  <div
    class="
      absolute
      bottom-10
      start-10
      z-10
      flex
      items-center
      gap-2

      bg-white
      shadow-lg
      p-2
      border-1
      border-gray-300
      rounded
      text-nowrap
    "
  >
    <label for="group-by-field" class="text-sm font-medium">Group by:</label>
    <select
      id="group-by-field"
      bind:value={selectedField}
      class="select max-w-xs"
    >
      <option value="">Select a field...</option>
      {#each varcharFields as field}
        <option value={field}>{field}</option>
      {/each}
    </select>
  </div>

  <div class="absolute bottom-10 right-10 z-10">
    <ViewSwitcher bind:view={currentView} views={['table', 'cards', 'rows']} />
  </div>

  {#if loading}
    <div class="flex items-center justify-center p-8">
      <span class="text-surface-500">Loading aggregation...</span>
    </div>
  {:else if error}
    <div class="alert preset-filled-error">
      <p>Error: {error}</p>
    </div>
  {:else if results.length > 0}
    {#if currentView === 'table'}
      <div class="table-container">
        <table class="table table-hover">
          <thead>
            <tr>
              <th>Field Value</th>
              <th class="text-end!">Occurrences</th>
            </tr>
          </thead>
          <tbody>
            {#each results as result}
              <tr>
                <!-- Display NULL/empty values as "None" -->
                <td>{result.value ?? 'None'}</td>
                <td class="text-right">
                  <button
                    type="button"
                    class="btn btn-sm hover:preset-tonal-primary"
                    onclick={() => handleCountClick(result.value)}
                  >
                    {result.count.toLocaleString()}
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if currentView === 'cards'}
      <div class="grid gap-4 grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
        {#each results as result}
          <div
            class="card preset-filled-surface-100-900 border-surface-200-800
            rounded-md border-2 divide-surface-200-800 w-full divide-y flex flex-col"
          >
            <header class="rounded-t-sm">
              <div class="h-[200px] preset-filled-surface-200-800 flex justify-center items-center relative">
                <MediaItem media={result.photoUrl ? { identifier: result.photoUrl, occurrenceID: '' } : undefined} />
              </div>
            </header>
            <article class="space-y-2 p-3">
              <div class="font-medium text-base truncate">
                {result.value || "[None]"}
              </div>
            </article>
            <footer class="flex flex-col">
              <button
                type="button"
                class="w-full text-sm text-surface-600-400 truncate border-2
                border-surface-200-800 grid grid-cols-2 hover:bg-surface-100"
                onclick={() => handleCountClick(result.value)}
              >
                <div class="p-1 text-center bg-surface-200-800">Occurrences</div>
                <div class="p-1 text-center">{result.count.toLocaleString()}</div>
              </button>
            </footer>
          </div>
        {/each}
      </div>
    {:else}
      <div class="grid gap-4 grid-cols-1">
        {#each results as result}
          <GroupRow
            groupValue={result.value}
            groupCount={result.count}
            {searchParams}
            fieldName={selectedField}
            onClick={occ => {
              if (!coreIdColumn) return;
              const coreId = occ[coreIdColumn as keyof Occurrence];
              if (typeof (coreId) === 'string' || typeof (coreId) === 'number') {
                selectedOccurrenceId = coreId;
                drawerOpen = true;
              }
            }}
            onCountClick={() => handleCountClick(result.value)}
          />
        {/each}
      </div>
    {/if}
  {:else if selectedField}
    <div class="flex items-center justify-center p-8">
      <span class="text-surface-500">No results found</span>
    </div>
  {/if}
</div>

<OccurrenceDrawer
  bind:open={drawerOpen}
  occurrenceId={selectedOccurrenceId}
  {coreIdColumn}
  onClose={() => { drawerOpen = false; }}
/>
