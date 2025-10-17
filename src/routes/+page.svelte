<script lang="ts">
  import { invoke, getCurrentWindow, listen, open } from '$lib/tauri-api';
  import { onMount } from "svelte";
  import { createVirtualizer, type Virtualizer } from '@tanstack/svelte-virtual';
  import Filters from '$lib/components/Filters.svelte';

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
  // Note: virtualizer is a store from TanStack Virtual, not wrapped in $state
  // to avoid infinite loops. Use virtualizerReady flag for initial render trigger.
  // Using type assertion (as) instead of type annotation (:) because when Svelte's
  // template compiler sees $virtualizer, it needs to unwrap the store type. With a
  // strict type annotation, TypeScript's inference fails and resolves to 'never'.
  // Type assertion allows TypeScript to infer the unwrapped type correctly.
  //
  // svelte-ignore non_reactive_update
  let virtualizer = null as ReturnType<typeof createVirtualizer<Element, Element>> | null;
  let virtualizerReady = $state(false);
  let virtualizerInitialized = $state(false);

  // Load a chunk of results from the backend and add them to the cache
  async function loadChunk(chunkIndex: number) {
    if (loadingChunks.has(chunkIndex)) {
      return; // Already loading this chunk
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
      console.log('[+page.svelte] searchResult', searchResult);

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

  // Tanstack Virtual onChange handler
  let lastLoadedRange = { firstChunk: -1, lastChunk: -1 };
  function loadVisibleChunks(instance: Virtualizer<Element, Element>) {
    const items = instance.getVirtualItems();
    if (items.length === 0) return;

    // Get the range of visible items
    const firstIndex = items[0].index;
    const lastIndex = items[items.length - 1].index;

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

    // Load all chunks in the visible range
    for (let chunk = firstChunk; chunk <= lastChunk; chunk++) {
      loadChunk(chunk);
    }
  }

  async function handleSearchChange(params: SearchParams) {
    if (!scrollElement) return;
    if (!archive) return;

    // Update search params
    currentSearchParams = params;

    // Clear cache
    occurrenceCache = new Map();
    occurrenceCacheVersion = 0;
    loadingChunks = new Set();

    // Load first chunk to get the filtered count
    try {
      const searchResult = await invoke<SearchResult>('search', {
        limit: CHUNK_SIZE,
        offset: 0,
        searchParams: params,
        fields: DISPLAY_FIELDS,
      });

      // Update filtered total
      filteredTotal = searchResult.total;

      // Add results to cache
      searchResult.results.forEach((occurrence, i) => {
        occurrenceCache.set(i, occurrence);
      });
      occurrenceCacheVersion++;

      // Scroll to top before creating new virtualizer
      if (scrollElement) {
        scrollElement.scrollTop = 0;
      }

      // Create new virtualizer with filtered count
      // Svelte's store subscription system will automatically update the template
      virtualizer = createVirtualizer({
        count: filteredTotal,
        getScrollElement: () => scrollElement ?? null,
        estimateSize: () => 40,
        overscan: 50,
        onChange: loadVisibleChunks,
      });

      virtualizerReady = true;
    } catch (e) {
      console.error('[+page.svelte] Error in handleSearchChange:', e);
    }
  }

  $effect(() => {
    // If we don't yet have an archive, there are no results to virtualize
    if (!archive) return;
    // If the scroll element that contains the virtualized results hasn't been
    // mounted yet, there's nothing to do
    if (!scrollElement) return;
    // If we've already created a virtualizer (or another has been created for
    // search results), don't recreate it
    if (virtualizer) return;

    // Initialize filteredTotal with full archive count
    filteredTotal = archive.coreCount;

    virtualizer = createVirtualizer({
      count: filteredTotal,
      getScrollElement: () => scrollElement ?? null,
      estimateSize: () => 40,
      overscan: 50,
      onChange: loadVisibleChunks,
    });

    virtualizerReady = true;
  });

  $effect(() => {
    if (archive) {
      getCurrentWindow().setTitle(`${archive.name} – ${archive.coreCount} records`);
    }
  })

  async function openArchive() {
    const path = await open();
    if (path) {
      archive = await invoke('open_archive', { path });
    }
  }

  onMount(() => {
    invoke('current_archive')
      .then(result => {
        archive = result as ArchiveInfo;
      })
      .catch(e => {
        // it's ok if there's no open archive
      });

    // Listen for menu events
    let unlisten: (() => void) | undefined;
    listen('menu-open', openArchive).then(unlistenFn => {
      unlisten = unlistenFn;
    });

    return () => {
      unlisten?.();
    };
  });
</script>

{#if archive}
  <div class="flex flex-row p-4 fixed w-full h-screen">
    <Filters onSearchChange={handleSearchChange} />
    <main class="overflow-y-auto w-full relative" bind:this={scrollElement}>
      {#if virtualizerReady && virtualizer}
        <div class="w-full">
          <div class="flex items-center py-2 px-2 border-b font-bold">
            <!-- <div class="w-16">#</div> -->
            <div class="flex-1">occurrenceID</div>
            <div class="flex-1">scientificName</div>
            <div class="w-24">lat</div>
            <div class="w-24">lng</div>
            <div class="w-32">eventDate</div>
            <div class="w-32">eventTime</div>
          </div>
          <div class="w-full relative" style="height: {$virtualizer!.getTotalSize()}px;">
            <!-- ensure this shows new data when we load new records -->
            {#key occurrenceCacheVersion}
              <div
                class="absolute top-0 left-0 w-full"
                style="transform: translateY({$virtualizer!.getVirtualItems()[0]?.start ?? 0}px);"
              >
                {#each $virtualizer!.getVirtualItems() as virtualRow (virtualRow.index)}
                  {@const occurrence = occurrenceCache.get(virtualRow.index)}
                <div
                  class="flex items-center py-2 px-2 border-b"
                  style="height: {virtualRow.size}px;"
                >
                  {#if occurrence}
                    <!-- <div class="w-16 truncate">{virtualRow.index}</div> -->
                    <div class="flex-1 truncate">{occurrence.occurrenceID}</div>
                    <div class="flex-1 truncate">{occurrence.scientificName}</div>
                    <div class="w-24 truncate">{occurrence.decimalLatitude}</div>
                    <div class="w-24 truncate">{occurrence.decimalLongitude}</div>
                    <div class="w-32 truncate">{occurrence.eventDate}</div>
                    <div class="w-32 truncate">{occurrence.eventTime}</div>
                  {:else}
                    <!-- <div class="w-16 truncate">{virtualRow.index}</div> -->
                    <div class="flex-1 text-gray-400">Loading...</div>
                  {/if}
                </div>
              {/each}
              </div>
            {/key}
          </div>
        </div>
      {:else}
        <div class="p-4">Loading...</div>
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
