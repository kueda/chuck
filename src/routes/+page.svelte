<script lang="ts">
import { Progress, Tabs } from '@skeletonlabs/skeleton-svelte';
import { Combine, Grid3x3, Rows4 } from 'lucide-svelte';
import { onMount } from 'svelte';
import Filters from '$lib/components/Filters.svelte';
import ViewSwitcher from '$lib/components/ViewSwitcher.svelte';
import {
  getCurrentWindow,
  invoke,
  listen,
  showOpenDialog,
} from '$lib/tauri-api';
import type { ArchiveInfo, Occurrence, SearchResult } from '$lib/types/archive';
import type { SearchParams } from '$lib/utils/filterCategories';
import {
  getColumnPreferences,
  getViewType,
  saveColumnPreferences,
  saveViewType,
} from '$lib/utils/viewPreferences';
import Cards from './Cards.svelte';
import Groups from './Groups.svelte';
import MapView from './Map.svelte';
import Table from './Table.svelte';

const CHUNK_SIZE = 500;

let archive = $state<ArchiveInfo>();
// Map is not a reactive data structure in Svelte, so we use
// occurrenceCacheVersion to trigger reactivity when the cache is updated.
// We ignore the non_reactive_update warning because we're intentionally
// managing reactivity manually via occurrenceCacheVersion.
//
// svelte-ignore non_reactive_update
let occurrenceCache = new Map<number, Occurrence>();
let occurrenceCacheVersion = $state(0);
let loadingChunks = new Set<number>();
let scrollElement: Element | undefined = $state(undefined);
let searchParams = $state<SearchParams>({});
let filteredTotal = $state<number>(0);
let currentView = $state<'table' | 'cards' | 'map'>(getViewType());
let lastVisibleRange = $state({
  firstIndex: 0,
  lastIndex: 0,
  scrollOffsetIndex: 0,
});

// Use a simple object ref that persists across reactivity
// const scrollState = { targetIndex: 0, shouldScroll: false };
let scrollState = $state({ targetIndex: 0, shouldScroll: false });

// Tab state
let activeTab = $state<string>('occurrences');

// Map state preservation
let mapCenter = $state<[number, number]>([0, 0]);
let mapZoom = $state(2);

// Drawer state (lifted to persist across view switches)
let drawerState = $state<{
  open: boolean;
  selectedOccurrenceId: string | number | null;
  selectedOccurrenceIndex: number | null;
}>({
  open: false,
  selectedOccurrenceId: null,
  selectedOccurrenceIndex: null,
});

let archiveLoadingStatus = $state<
  null | 'importing' | 'extracting' | 'creatingDatabase'
>(null);
const archiveLoadingProgress = $derived.by(() => {
  switch (archiveLoadingStatus) {
    case null:
      return 10;
    case 'importing':
      return 20;
    case 'extracting':
      return 40;
    case 'creatingDatabase':
      return 60;
    default:
      return 0;
  }
});
let archiveLoadingError = $state<string | null>(null);

// Column visibility state
let visibleColumns = $state<string[]>([]);

// Fields to fetch from backend (always includes core ID even if not visible)
const fetchedFields = $derived.by(() => {
  if (!archive || visibleColumns.length === 0) return visibleColumns;
  const coreIdColumn = archive.coreIdColumn;
  return [
    coreIdColumn,
    ...visibleColumns.filter((col) => col !== coreIdColumn),
  ];
});

// Initialize visible columns when archive loads
$effect(() => {
  if (archive) {
    const columns = getColumnPreferences(
      archive.name,
      archive.coreIdColumn,
      archive.availableColumns,
    );
    visibleColumns = columns;
  }
});

// Get VARCHAR fields for grouping (exclude DATE/numeric types)
const varcharFields = $derived(
  archive
    ? archive.availableColumns
        .filter((col) => {
          // Exclude known non-VARCHAR types
          const numericFields = [
            'decimalLatitude',
            'decimalLongitude',
            'coordinateUncertaintyInMeters',
          ];
          const dateFields = ['eventDate'];
          return !numericFields.includes(col) && !dateFields.includes(col);
        })
        .sort()
    : [],
);

// Handle column visibility changes
function handleVisibleColumnsChange(newColumns: string[]) {
  if (!archive) return;

  // Save preferences
  saveColumnPreferences(archive.name, newColumns);

  // Preserve scroll position
  scrollState.targetIndex = lastVisibleRange.firstIndex;
  scrollState.shouldScroll = true;

  // Update visible columns
  visibleColumns = newColumns;

  // Clear cache and reload
  occurrenceCache = new Map();
  loadingChunks = new Set();
  lastLoadedRange = { firstChunk: -1, lastChunk: -1 };
  occurrenceCacheVersion++;

  // Trigger reload of visible chunks
  const firstChunk = Math.floor(lastVisibleRange.firstIndex / CHUNK_SIZE);
  const lastChunk = Math.floor(lastVisibleRange.lastIndex / CHUNK_SIZE);
  for (let chunk = firstChunk; chunk <= lastChunk; chunk++) {
    loadChunk(chunk);
  }
}

// Load a chunk of results from the backend and add them to the cache
async function loadChunk(chunkIndex: number) {
  if (loadingChunks.has(chunkIndex)) {
    return;
  }

  const offset = chunkIndex * CHUNK_SIZE;

  // Don't load chunks beyond the filtered total
  if (offset >= filteredTotal) {
    return;
  }

  loadingChunks.add(chunkIndex);

  try {
    const searchResult = await invoke<SearchResult>('search', {
      limit: CHUNK_SIZE,
      offset: offset,
      searchParams: searchParams,
      fields: fetchedFields,
    });

    // Add results to cache
    searchResult.results.forEach((occurrence, i) => {
      occurrenceCache.set(offset + i, occurrence);
    });

    // Trigger reactivity by incrementing version counter
    occurrenceCacheVersion++;
  } catch (e) {
    console.error(`[+page.svelte] Error loading chunk ${chunkIndex}:`, e);
  } finally {
    loadingChunks.delete(chunkIndex);
  }
}

// Handler for when visible range changes in virtualizer
let lastLoadedRange = { firstChunk: -1, lastChunk: -1 };
function handleVisibleRangeChange({
  firstIndex,
  lastIndex,
  scrollOffsetIndex,
}: {
  firstIndex: number;
  lastIndex: number;
  scrollOffsetIndex: number | undefined;
}) {
  // Track the current visible range for view switching
  if (scrollOffsetIndex && scrollOffsetIndex > 0) {
    lastVisibleRange = { firstIndex, lastIndex, scrollOffsetIndex };
  }

  // Calculate which chunks we need
  const firstChunk = Math.floor(firstIndex / CHUNK_SIZE);
  const lastChunk = Math.floor(lastIndex / CHUNK_SIZE);

  // Skip if we're already loading this exact range. Performance will suffer
  // a lot for large archives without this
  if (
    firstChunk === lastLoadedRange.firstChunk &&
    lastChunk === lastLoadedRange.lastChunk
  ) {
    return;
  }

  lastLoadedRange = { firstChunk, lastChunk };

  for (let chunk = firstChunk; chunk <= lastChunk; chunk++) {
    loadChunk(chunk);
  }
}

function handleColumnHeaderClick(column: string) {
  if (!archive) return;

  // Toggle between ASC and DESC (no "clear" state since results are always sorted)
  let newDirection: 'ASC' | 'DESC' = 'ASC';

  if (searchParams.sort_by === column) {
    // Same column - toggle direction
    newDirection = searchParams.sort_direction === 'ASC' ? 'DESC' : 'ASC';
  }
  // Different column - start with ASC

  const params: SearchParams = {
    ...searchParams,
    sort_by: column,
    sort_direction: newDirection,
  };

  handleSearchChange(params);
}

async function handleSearchChange(params: SearchParams) {
  if (!scrollElement) return;
  if (!archive) return;

  // Update search params immediately so subsequent clicks see the new value
  searchParams = params;

  // Load first chunk with new params to get the filtered count
  try {
    const searchResult = await invoke<SearchResult>('search', {
      limit: CHUNK_SIZE,
      offset: 0,
      searchParams: params,
      fields: fetchedFields,
    });

    // Now that we have the results, update the rest atomically
    filteredTotal = searchResult.total;

    // Clear cache
    occurrenceCache = new Map();
    loadingChunks = new Set();
    lastLoadedRange = { firstChunk: -1, lastChunk: -1 };

    // Add results to cache
    searchResult.results.forEach((occurrence, i) => {
      occurrenceCache.set(i, occurrence);
    });

    // Increment version to force re-render (don't reset to 0!)
    occurrenceCacheVersion++;

    // Scroll to top
    if (scrollElement) {
      scrollElement.scrollTop = 0;
    }
  } catch (e) {
    console.error('[+page.svelte] Error in handleSearchChange:', e);
  }
}

// Set window title and initialize filtered total when archive loads
$effect(() => {
  if (archive) {
    getCurrentWindow().setTitle(
      `${archive.name} – ${archive.coreCount} occurrences`,
    );
    filteredTotal = archive.coreCount;

    // Set default sort to Core ID
    searchParams = {
      sort_by: archive.coreIdColumn,
      sort_direction: 'ASC',
    };
  }
});

// Persist view preference to localStorage and capture scroll position for view switch
let previousView: 'table' | 'cards' | 'map' | null = null;
$effect(() => {
  saveViewType(currentView);
});

function onViewChange() {
  // When view changes, capture the scroll target
  if (previousView !== currentView && lastVisibleRange.scrollOffsetIndex > 0) {
    scrollState.targetIndex = lastVisibleRange.scrollOffsetIndex;
    scrollState.shouldScroll = true;
  }
  previousView = currentView;
}

function handleMapMove(center: [number, number], zoom: number) {
  mapCenter = center;
  mapZoom = zoom;
}

function clearArchiveData() {
  occurrenceCache = new Map();
  occurrenceCacheVersion = 0;
  loadingChunks = new Set();
  searchParams = {};
  lastLoadedRange = { firstChunk: -1, lastChunk: -1 };
  archiveLoadingStatus = 'importing';
  archiveLoadingError = null;
}

async function openArchive() {
  const path = await showOpenDialog();
  if (!path) return;

  // Clear existing data before opening new archive
  clearArchiveData();

  try {
    // Open the new archive (progress will be reported via events)
    archive = await invoke('open_archive', { path });

    // Clear loading status on success
    archiveLoadingStatus = null;

    // Reset scroll position
    if (scrollElement) {
      scrollElement.scrollTop = 0;
    }
  } catch (e) {
    archiveLoadingError = e instanceof Error ? e.message : String(e);
    archiveLoadingStatus = null;
    console.error('[+page.svelte] Error opening archive:', e);
  }
}

onMount(() => {
  invoke('current_archive')
    .then((result) => {
      archive = result as ArchiveInfo;
    })
    .catch((_e) => {
      // it's ok if there's no open archive
    });

  // Listen for menu events
  let unlistenMenu: (() => void) | undefined;
  listen('menu-open', openArchive).then((unlistenFn) => {
    unlistenMenu = unlistenFn;
  });

  // Listen for archive open progress events
  type ProgressEvent =
    | { status: 'importing' }
    | { status: 'extracting' }
    | { status: 'creatingDatabase' }
    | { status: 'complete'; info: ArchiveInfo }
    | { status: 'error'; message: string };

  let unlistenProgress: (() => void) | undefined;
  listen<ProgressEvent>('archive-open-progress', (event) => {
    const progress = event.payload;

    // Clear existing data. When manually opening an archive we may have
    // already done this, but if the backend opens the archive for another
    // reason (e.g. after downloading one from iNat), we still want to
    // clear things out
    clearArchiveData();

    switch (progress.status) {
      case 'importing':
        archiveLoadingStatus = 'importing';
        archiveLoadingError = null;
        break;
      case 'extracting':
        archiveLoadingStatus = 'extracting';
        break;
      case 'creatingDatabase':
        archiveLoadingStatus = 'creatingDatabase';
        break;
      case 'complete':
        archiveLoadingStatus = null;
        archiveLoadingError = null;
        // Update archive info with the newly opened archive
        archive = progress.info;
        break;
      case 'error':
        archiveLoadingStatus = null;
        archiveLoadingError = progress.message;
        console.error('[+page.svelte] Archive open error:', progress.message);
        break;
    }
  }).then((unlistenFn) => {
    unlistenProgress = unlistenFn;
  });

  return () => {
    unlistenMenu?.();
    unlistenProgress?.();
  };
});
</script>

{#if archiveLoadingStatus}
  <div
    class="w-full h-screen flex flex-col justify-center items-center p-4 text-center"
  >
    <div class="mb-4">
      <div class="text-xl mb-2">
        {#if archiveLoadingStatus === 'importing'}
          Importing archive...
        {:else if archiveLoadingStatus === 'extracting'}
          Extracting archive...
        {:else if archiveLoadingStatus === 'creatingDatabase'}
          Creating database...
        {/if}
      </div>
      <div class="text-sm text-gray-500">
        This may take a few moments for large archives
      </div>
    </div>
    <Progress value={archiveLoadingProgress} class="w-64">
      <Progress.Track>
        <Progress.Range />
      </Progress.Track>
    </Progress>

  </div>
{:else if archiveLoadingError}
  <div
    class="w-full h-screen flex flex-col justify-center items-center p-4 text-center"
  >
    <div class="text-xl mb-4 text-red-600">Error Opening Archive</div>
    <div class="text-sm mb-6 max-w-md">{archiveLoadingError}</div>
    <button
      type="button"
      class="btn preset-filled"
      onclick={() => {
        archiveLoadingError = null;
        openArchive();
      }}
    >
      Try Again
    </button>
  </div>
{:else if archive}
  <div class="flex flex-row p-4 fixed w-full h-screen">
    <aside class="pe-4 w-82 overflow-y-auto">
      <Filters
        onSearchChange={handleSearchChange}
        availableColumns={archive?.availableColumns ?? []}
        {searchParams}
      />
    </aside>
    <main class="overflow-hidden w-full relative flex flex-col">
      <Tabs
        value={activeTab}
        onValueChange={(details) => (activeTab = details.value)}
        class="flex h-full flex-col"
      >
        <Tabs.List class="border-b border-surface-200-800 px-4 mb-0 flex flex-row">
          <Tabs.Trigger value="occurrences" class="btn hover:preset-tonal grow">
            <Grid3x3 size={18} />
            Occurrences
          </Tabs.Trigger>
          <Tabs.Trigger value="groups" class="btn hover:preset-tonal grow">
            <Combine size={18} />
            Groups
          </Tabs.Trigger>
          <Tabs.Indicator class="bg-surface-950-50" />
        </Tabs.List>

        <Tabs.Content value="occurrences" class="flex-1 overflow-hidden">
          <div class="h-full overflow-y-auto" bind:this={scrollElement} data-testid="occurrences-scroll-container">
            {#if currentView === 'table'}
              <Table
                {drawerState}
                {occurrenceCache}
                {occurrenceCacheVersion}
                count={filteredTotal}
                {scrollElement}
                onVisibleRangeChange={handleVisibleRangeChange}
                coreIdColumn={archive.coreIdColumn}
                archiveName={archive.name}
                availableColumns={archive.availableColumns}
                {visibleColumns}
                {scrollState}
                currentSortColumn={searchParams.sort_by}
                currentSortDirection={searchParams.sort_direction}
                onColumnHeaderClick={handleColumnHeaderClick}
                onVisibleColumnsChange={handleVisibleColumnsChange}
              />
            {:else if currentView === 'cards'}
              <Cards
                {drawerState}
                {occurrenceCache}
                {occurrenceCacheVersion}
                count={filteredTotal}
                {scrollElement}
                onVisibleRangeChange={handleVisibleRangeChange}
                coreIdColumn={archive.coreIdColumn}
                {scrollState}
              />
            {:else if currentView === 'map'}
              <MapView
                {drawerState}
                coreIdColumn={archive.coreIdColumn}
                params={searchParams}
                center={mapCenter}
                zoom={mapZoom}
                onMapMove={handleMapMove}
                onBoundsChange={(bounds) => {
                  if (bounds) {
                    const params = {
                      ...searchParams,
                      nelat: String(bounds.nelat),
                      nelng: String(bounds.nelng),
                      swlat: String(bounds.swlat),
                      swlng: String(bounds.swlng)
                    };
                    handleSearchChange(params);
                  } else {
                    const { nelat, nelng, swlat, swlng, ...rest} = searchParams;
                    handleSearchChange(rest);
                  }
                }}
              />
            {/if}
          </div>
          <div class="absolute bottom-10 right-10 z-10">
            <ViewSwitcher bind:view={currentView} {onViewChange} />
          </div>
        </Tabs.Content>

        <Tabs.Content value="groups" class="flex-1 overflow-auto">
          {#if activeTab === 'groups'}
            <Groups
              coreIdColumn={archive.coreIdColumn}
              {searchParams}
              {varcharFields}
              defaultSelectedField={
                archive?.availableColumns?.includes('scientificName') ? 'scientificName' : undefined}
              onCountClick={(fieldName: string, fieldValue: string | null) => {
                const params: SearchParams = {
                  ...searchParams,
                  [fieldName]: fieldValue ?? ''
                };
                handleSearchChange(params);
                activeTab = 'occurrences';
              }}
            />
          {/if}
        </Tabs.Content>
      </Tabs>
    </main>
  </div>
{:else}
  <div class="w-full h-screen flex flex-col justify-center items-center p-4 text-center">
    <p class="w-3/4 mb-5">Chuck is an application for viewing archives of biodiversity occurrences called DarwinCore Archives. Open an existing archive to get started</p>
    <button
      type="button"
      class="btn preset-filled"
      onclick={openArchive}
    >
      Open Archive
    </button>
  </div>
{/if}
