<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from "svelte";
  import { createVirtualizer, type Virtualizer } from '@tanstack/svelte-virtual';
  import Filters from '$lib/components/Filters.svelte';

  import type { SearchParams } from '$lib/components/Filters.svelte';

  interface ArchiveInfo {
    name: string,
    coreCount: number,
  }

  // Attributes are undefined when the client doesn't ask for them, and
  // (hopefully) null when the client asks for them but they are blank
  interface Occurrence {
    occurrenceID?: string | null;
    basisOfRecord?: string | null;
    recordedBy?: string | null;
    eventDate?: string | null;
    decimalLatitude?: number | null;
    decimalLongitude?: number | null;
    scientificName?: string | null;
    taxonRank?: string | null;
    taxonomicStatus?: string | null;
    vernacularName?: string | null;
    kingdom?: string | null;
    phylum?: string | null;
    class?: string | null;
    order?: string | null;
    family?: string | null;
    genus?: string | null;
    specificEpithet?: string | null;
    infraspecificEpithet?: string | null;
    taxonID?: string | null;
    occurrenceRemarks?: string | null;
    establishmentMeans?: string | null;
    georeferencedDate?: string | null;
    georeferenceProtocol?: string | null;
    coordinateUncertaintyInMeters?: string | null;
    coordinatePrecision?: string | null;
    geodeticDatum?: string | null;
    accessRights?: string | null;
    license?: string | null;
    informationWithheld?: string | null;
    modified?: string | null;
    captive?: string | null;
    eventTime?: string | null;
    verbatimEventDate?: string | null;
    verbatimLocality?: string | null;
    continent?: string | null;
    countryCode?: string | null;
    stateProvince?: string | null;
    county?: string | null;
    municipality?: string | null;
    locality?: string | null;
    waterBody?: string | null;
    island?: string | null;
    islandGroup?: string | null;
    elevation?: string | null;
    elevationAccuracy?: string | null;
    depth?: string | null;
    depthAccuracy?: string | null;
    minimumDistanceAboveSurfaceInMeters?: string | null;
    maximumDistanceAboveSurfaceInMeters?: string | null;
    habitat?: string | null;
    georeferenceRemarks?: string | null;
    georeferenceSources?: string | null;
    georeferenceVerificationStatus?: string | null;
    georeferencedBy?: string | null;
    pointRadiusSpatialFit?: string | null;
    footprintSpatialFit?: string | null;
    footprintWKT?: string | null;
    footprintSRS?: string | null;
    verbatimSRS?: string | null;
    verbatimCoordinateSystem?: string | null;
    verticalDatum?: string | null;
    verbatimElevation?: string | null;
    verbatimDepth?: string | null;
    distanceFromCentroidInMeters?: string | null;
    hasCoordinate?: string | null;
    hasGeospatialIssues?: string | null;
    higherGeography?: string | null;
    higherGeographyID?: string | null;
    locationAccordingTo?: string | null;
    locationID?: string | null;
    locationRemarks?: string | null;
    year?: string | null;
    month?: string | null;
    day?: string | null;
    startDayOfYear?: string | null;
    endDayOfYear?: string | null;
    eventID?: string | null;
    parentEventID?: string | null;
    eventType?: string | null;
    eventRemarks?: string | null;
    samplingEffort?: string | null;
    samplingProtocol?: string | null;
    sampleSizeValue?: string | null;
    sampleSizeUnit?: string | null;
    fieldNotes?: string | null;
    fieldNumber?: string | null;
    acceptedScientificName?: string | null;
    acceptedNameUsage?: string | null;
    acceptedNameUsageID?: string | null;
    higherClassification?: string | null;
    subfamily?: string | null;
    subgenus?: string | null;
    tribe?: string | null;
    subtribe?: string | null;
    superfamily?: string | null;
    species?: string | null;
    genericName?: string | null;
    infragenericEpithet?: string | null;
    cultivarEpithet?: string | null;
    parentNameUsage?: string | null;
    parentNameUsageID?: string | null;
    originalNameUsage?: string | null;
    originalNameUsageID?: string | null;
    namePublishedIn?: string | null;
    namePublishedInID?: string | null;
    namePublishedInYear?: string | null;
    nomenclaturalCode?: string | null;
    nomenclaturalStatus?: string | null;
    nameAccordingTo?: string | null;
    nameAccordingToID?: string | null;
    taxonConceptID?: string | null;
    scientificNameID?: string | null;
    taxonRemarks?: string | null;
    taxonomicIssue?: string | null;
    nonTaxonomicIssue?: string | null;
    associatedTaxa?: string | null;
    verbatimIdentification?: string | null;
    verbatimTaxonRank?: string | null;
    verbatimScientificName?: string | null;
    typifiedName?: string | null;
    identifiedBy?: string | null;
    identifiedByID?: string | null;
    dateIdentified?: string | null;
    identificationID?: string | null;
    identificationQualifier?: string | null;
    identificationReferences?: string | null;
    identificationRemarks?: string | null;
    identificationVerificationStatus?: string | null;
    previousIdentifications?: string | null;
    typeStatus?: string | null;
    institutionCode?: string | null;
    institutionID?: string | null;
    collectionCode?: string | null;
    collectionID?: string | null;
    ownerInstitutionCode?: string | null;
    catalogNumber?: string | null;
    recordNumber?: string | null;
    otherCatalogNumbers?: string | null;
    preparations?: string | null;
    disposition?: string | null;
    organismID?: string | null;
    organismName?: string | null;
    organismQuantity?: string | null;
    organismQuantityType?: string | null;
    relativeOrganismQuantity?: string | null;
    organismRemarks?: string | null;
    organismScope?: string | null;
    associatedOrganisms?: string | null;
    individualCount?: string | null;
    lifeStage?: string | null;
    sex?: string | null;
    reproductiveCondition?: string | null;
    behavior?: string | null;
    caste?: string | null;
    vitality?: string | null;
    degreeOfEstablishment?: string | null;
    pathway?: string | null;
    isInvasive?: string | null;
    materialSampleID?: string | null;
    materialEntityID?: string | null;
    materialEntityRemarks?: string | null;
    associatedOccurrences?: string | null;
    associatedSequences?: string | null;
    associatedReferences?: string | null;
    isSequenced?: string | null;
    occurrenceStatus?: string | null;
    bibliographicCitation?: string | null;
    references?: string | null;
    language?: string | null;
    rightsHolder?: string | null;
    dataGeneralizations?: string | null;
    dynamicProperties?: string | null;
    type?: string | null;
    datasetID?: string | null;
    datasetName?: string | null;
    issue?: string | null;
    mediaType?: string | null;
    projectId?: string | null;
    protocol?: string | null;
    geologicalContextID?: string | null;
    bed?: string | null;
    formation?: string | null;
    group?: string | null;
    member?: string | null;
    lithostratigraphicTerms?: string | null;
    earliestEonOrLowestEonothem?: string | null;
    latestEonOrHighestEonothem?: string | null;
    earliestEraOrLowestErathem?: string | null;
    latestEraOrHighestErathem?: string | null;
    earliestPeriodOrLowestSystem?: string | null;
    latestPeriodOrHighestSystem?: string | null;
    earliestEpochOrLowestSeries?: string | null;
    latestEpochOrHighestSeries?: string | null;
    earliestAgeOrLowestStage?: string | null;
    latestAgeOrHighestStage?: string | null;
    lowestBiostratigraphicZone?: string | null;
    highestBiostratigraphicZone?: string | null;
    gbifID?: string | null;
    gbifRegion?: string | null;
    taxonKey?: string | null;
    acceptedTaxonKey?: string | null;
    kingdomKey?: string | null;
    phylumKey?: string | null;
    classKey?: string | null;
    orderKey?: string | null;
    familyKey?: string | null;
    genusKey?: string | null;
    subgenusKey?: string | null;
    speciesKey?: string | null;
    datasetKey?: string | null;
    publisher?: string | null;
    publishingCountry?: string | null;
    publishedByGbifRegion?: string | null;
    lastCrawled?: string | null;
    lastParsed?: string | null;
    lastInterpreted?: string | null;
    iucnRedListCategory?: string | null;
    repatriated?: string | null;
    level0Gid?: string | null;
    level0Name?: string | null;
    level1Gid?: string | null;
    level1Name?: string | null;
    level2Gid?: string | null;
    level2Name?: string | null;
    level3Gid?: string | null;
    level3Name?: string | null;
    recordedByID?: string | null;
    verbatimLabel?: string | null;
  };

  interface SearchResult {
    total: number;
    results: Occurrence[];
  }

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
