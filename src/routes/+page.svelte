<script lang="ts">
  import { onMount } from 'svelte';
  import { Progress } from '@skeletonlabs/skeleton-svelte';
  import { invoke, getCurrentWindow, listen, showOpenDialog } from '$lib/tauri-api';
  import Filters from '$lib/components/Filters.svelte';
  import ViewSwitcher from '$lib/components/ViewSwitcher.svelte';

  import Cards from './Cards.svelte';
  import Table from './Table.svelte';

  import type { SearchParams } from '$lib/components/Filters.svelte';
  import type { ArchiveInfo, Occurrence, SearchResult } from '$lib/types/archive';

  const CHUNK_SIZE = 500;
  const DISPLAY_FIELDS = [
    'occurrenceID',
    'scientificName',
    'decimalLatitude',
    'decimalLongitude',
    'eventDate',
    'eventTime'
  ];

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
  let currentSearchParams = $state<SearchParams>({});
  let filteredTotal = $state<number>(0);
  let currentView = $state<'table' | 'cards'>('table');
  let lastVisibleRange = $state({ firstIndex: 0, lastIndex: 0, scrollOffsetIndex: 0 });

  // Use a simple object ref that persists across reactivity
  // const scrollState = { targetIndex: 0, shouldScroll: false };
  let scrollState = $state({ targetIndex: 0, shouldScroll: false });
  let archiveLoadingStatus = $state<
    null | 'importing' | 'extracting' | 'creatingDatabase'
  >(null);
  let archiveLoadingProgress = $derived.by(() => {
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
        searchParams: currentSearchParams,
        fields: DISPLAY_FIELDS,
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
  function handleVisibleRangeChange(
    {
      firstIndex,
      lastIndex,
      scrollOffsetIndex
    }: {
      firstIndex: number,
      lastIndex: number,
      scrollOffsetIndex: number | undefined,
    }
  ) {
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

  async function handleSearchChange(params: SearchParams) {
    if (!scrollElement) return;
    if (!archive) return;

    // Update search params
    currentSearchParams = params;

    // Set filtered total to 0 immediately to prevent virtualizer from
    // creating virtual items with stale count while cache is empty
    filteredTotal = 0;

    // Clear cache
    occurrenceCache = new Map();
    occurrenceCacheVersion = 0;
    loadingChunks = new Set();
    lastLoadedRange = { firstChunk: -1, lastChunk: -1 };

    // Load first chunk to get the filtered count
    try {
      const searchResult = await invoke<SearchResult>('search', {
        limit: CHUNK_SIZE,
        offset: 0,
        searchParams: params,
        fields: DISPLAY_FIELDS,
      });

      // Update filtered total with actual count
      filteredTotal = searchResult.total;

      // Add results to cache
      searchResult.results.forEach((occurrence, i) => {
        occurrenceCache.set(i, occurrence);
      });
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
      getCurrentWindow().setTitle(`${archive.name} – ${archive.coreCount} records`);
      filteredTotal = archive.coreCount;
    }
  });

  // Persist view preference to localStorage and capture scroll position for view switch
  let previousView: 'table' | 'cards' | null = null;
  $effect(() => {
    if (typeof window !== 'undefined') {
      localStorage.setItem('chuck:viewPreference', currentView);
    }
  });

  function onViewChange() {
    // When view changes, capture the scroll target
    if (
      previousView !== currentView
      && lastVisibleRange.scrollOffsetIndex > 0
    ) {
      scrollState.targetIndex = lastVisibleRange.scrollOffsetIndex;
      scrollState.shouldScroll = true;
    }
    previousView = currentView;
  }

  function clearArchiveData() {
    occurrenceCache = new Map();
    occurrenceCacheVersion = 0;
    loadingChunks = new Set();
    currentSearchParams = {};
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
    // Load saved view preference
    if (typeof window !== 'undefined') {
      const savedView = localStorage.getItem('chuck:viewPreference');
      if (savedView === 'table' || savedView === 'cards') {
        currentView = savedView;
      }
    }

    invoke('current_archive')
      .then(result => {
        archive = result as ArchiveInfo;
      })
      .catch(e => {
        // it's ok if there's no open archive
      });

    // Listen for menu events
    let unlistenMenu: (() => void) | undefined;
    listen('menu-open', openArchive).then(unlistenFn => {
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
    }).then(unlistenFn => {
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
    <div class="absolute bottom-10 right-10 z-10">
      <ViewSwitcher bind:view={currentView} {onViewChange} />
    </div>
    <Filters onSearchChange={handleSearchChange} />
    <main class="overflow-y-auto w-full relative" bind:this={scrollElement}>
      {#if currentView === 'table'}
        <Table
          {occurrenceCache}
          {occurrenceCacheVersion}
          count={filteredTotal}
          {scrollElement}
          onVisibleRangeChange={handleVisibleRangeChange}
          coreIdColumn={archive.coreIdColumn}
          {scrollState}
        />
      {:else}
        <Cards
          {occurrenceCache}
          {occurrenceCacheVersion}
          count={filteredTotal}
          {scrollElement}
          onVisibleRangeChange={handleVisibleRangeChange}
          coreIdColumn={archive.coreIdColumn}
          {scrollState}
        />
      {/if}
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
