<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from "svelte";
  import { createVirtualizer } from '@tanstack/svelte-virtual';

  interface ArchiveInfo {
    name: string,
    coreCount: number,
  }

  interface Occurrence {
    occurrenceID: string,
    scientificName: string,
    decimalLatitude: number,
    decimalLongitude: number,
    eventDate: Date,
    eventTime: Date,
  }

  const CHUNK_SIZE = 500;

  let archive = $state<ArchiveInfo>();
  let occurrenceCache = new Map<number, Occurrence>();
  let cacheVersion = $state(0);
  let loadingChunks = new Set<number>();
  let scrollElement: Element | undefined = $state(undefined);
  // Note: virtualizer is a store from TanStack Virtual, not wrapped in $state
  // to avoid infinite loops. Use virtualizerReady flag for initial render trigger.
  // Using type assertion (as) instead of type annotation (:) because when Svelte's
  // template compiler sees $virtualizer, it needs to unwrap the store type. With a
  // strict type annotation, TypeScript's inference fails and resolves to 'never'.
  // Type assertion allows TypeScript to infer the unwrapped type correctly.
  // svelte-ignore non_reactive_update
  let virtualizer = null as ReturnType<typeof createVirtualizer<Element, Element>> | null;
  let virtualizerReady = $state(false);

  async function loadChunk(chunkIndex: number) {
    if (loadingChunks.has(chunkIndex)) {
      return; // Already loading this chunk
    }

    loadingChunks.add(chunkIndex);
    const offset = chunkIndex * CHUNK_SIZE;

    try {
      const results = await invoke<Occurrence[]>('search', {
        limit: CHUNK_SIZE,
        offset: offset,
      });

      // Add results to cache
      results.forEach((occurrence, i) => {
        occurrenceCache.set(offset + i, occurrence);
      });
      // Trigger reactivity by incrementing version counter
      cacheVersion++;
    } catch (e) {
      console.error(`[+page.svelte] Error loading chunk ${chunkIndex}:`, e);
    } finally {
      loadingChunks.delete(chunkIndex);
    }
  }

  $effect(() => {
    if (archive && scrollElement) {
      const loadVisibleChunks = (instance: { getVirtualItems: () => Array<{ index: number }> }) => {
        const items = instance.getVirtualItems();
        if (items.length === 0) return;

        // Get the range of visible items
        const firstIndex = items[0].index;
        const lastIndex = items[items.length - 1].index;

        // Calculate which chunks we need
        const firstChunk = Math.floor(firstIndex / CHUNK_SIZE);
        const lastChunk = Math.floor(lastIndex / CHUNK_SIZE);

        // Load all chunks in the visible range
        for (let chunk = firstChunk; chunk <= lastChunk; chunk++) {
          loadChunk(chunk);
        }
      };

      virtualizer = createVirtualizer({
        count: archive.coreCount,
        getScrollElement: () => scrollElement ?? null,
        estimateSize: () => 40,
        overscan: 5,
        onChange: loadVisibleChunks
      });

      virtualizerReady = true;
    }
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
    <aside class="mr-4">side bar</aside>
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
            {#key cacheVersion}
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
