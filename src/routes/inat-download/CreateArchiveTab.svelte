<script lang="ts">
import { SegmentedControl } from '@skeletonlabs/skeleton-svelte';
import InatPlaceChooser from '$lib/components/InatPlaceChooser.svelte';
import InatTaxonChooser from '$lib/components/InatTaxonChooser.svelte';
import InatUserChooser from '$lib/components/InatUserChooser.svelte';
import { invoke, showSaveDialog } from '$lib/tauri-api';
import ExtensionCheckbox from './ExtensionCheckbox.svelte';
import {
  BYTES_PER_OBSERVATION,
  BYTES_PER_OBSERVATION_COMMENTS,
  BYTES_PER_OBSERVATION_IDENTIFICATIONS,
  BYTES_PER_OBSERVATION_MULTIMEDIA,
  BYTES_PER_PHOTO,
  BYTES_PER_SOUND,
  formatBytes,
  ONE_GB,
} from './size-estimate';

interface Props {
  ondownloadstart: (path: string, params: GenerateParams) => void;
}

const { ondownloadstart }: Props = $props();

// Keep in sync w/ src-tauri/commands/inat_download.rs
interface CountParams {
  taxon_id: number | null;
  place_id: number | null;
  user: string | null;
  d1: string | null;
  d2: string | null;
  created_d1: string | null;
  created_d2: string | null;
  url_params: string | null;
}

export interface GenerateParams {
  output_path: string;
  taxon_id: number | null;
  place_id: number | null;
  user: string | null;
  d1: string | null;
  d2: string | null;
  created_d1: string | null;
  created_d2: string | null;
  url_params: string | null;
  fetch_media: boolean;
  extensions: string[];
}

interface MediaEstimate {
  photo_count: number;
  sound_count: number;
  sample_size: number;
}

let filterMode = $state<'fields' | 'url'>('fields');
let urlInput = $state('');
let effectiveParams = $state('');
let urlParseError = $state(false);

let taxonId = $state<number | null>(null);
let placeId = $state<number | null>(null);
let userId = $state<number | null>(null);
let observedDateRange = $state<'all' | 'custom'>('all');
// This awkwardness is brought to you by Safari not actually showing a blank
// date input when the input value is blank. Instead it shows you the
// current date as a placeholder, but the input has no value until you
// choose a date with the UI or manually fill out *all* three date parts.
// So: better to set it to an actual date by default then mislead the user
// into thinking a date is chosen when it isn't
let observedD1 = $state<string>('2000-01-01');
let observedD2 = $state<string>(new Date().toDateString());
let createdDateRange = $state<'all' | 'custom'>('all');
let createdD1 = $state<string>('2000-01-01');
let createdD2 = $state<string>(new Date().toDateString());
let fetchMedia = $state<boolean>(false);
let includeSimpleMultimedia = $state<boolean>(true);
let includeAudiovisual = $state<boolean>(false);
let includeIdentifications = $state<boolean>(true);
let includeComments = $state<boolean>(true);

let observationCount = $state<number | null>(null);
let countLoading = $state<boolean>(false);
let countError = $state<string | null>(null);

let mediaEstimate = $state<MediaEstimate | null>(null);
let mediaEstimateLoading = $state<boolean>(false);

let showLargeDownloadDialog = $state<boolean>(false);
let pendingDownloadPath = $state<string | null>(null);

const DEBOUNCE_MS = 500;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let photoDebounceTimer: ReturnType<typeof setTimeout> | null = null;

function buildCountParams(): CountParams {
  return filterMode === 'url'
    ? {
        taxon_id: null,
        place_id: null,
        user: null,
        d1: null,
        d2: null,
        created_d1: null,
        created_d2: null,
        url_params: effectiveParams || null,
      }
    : {
        taxon_id: taxonId,
        place_id: placeId,
        user: userId ? userId.toString() : null,
        d1: observedDateRange === 'custom' && observedD1 ? observedD1 : null,
        d2: observedDateRange === 'custom' && observedD2 ? observedD2 : null,
        created_d1:
          createdDateRange === 'custom' && createdD1 ? createdD1 : null,
        created_d2:
          createdDateRange === 'custom' && createdD2 ? createdD2 : null,
        url_params: null,
      };
}

async function fetchCount() {
  if (filterMode === 'url' && !effectiveParams) {
    observationCount = null;
    countLoading = false;
    countError = null;
    return;
  }
  countLoading = true;
  countError = null;
  try {
    observationCount = await invoke<number>('get_observation_count', {
      params: buildCountParams(),
    });
  } catch (e) {
    console.error('Failed to fetch count:', e);
    countError = 'Unable to load observation count';
    observationCount = null;
  } finally {
    countLoading = false;
  }
}

function scheduleFetchCount() {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    fetchCount();
  }, DEBOUNCE_MS);
}

async function fetchMediaEstimate() {
  if (!fetchMedia || (filterMode === 'url' && !effectiveParams)) {
    mediaEstimate = null;
    return;
  }
  mediaEstimateLoading = true;
  try {
    mediaEstimate = await invoke<MediaEstimate>('estimate_media_count', {
      params: buildCountParams(),
    });
  } catch (e) {
    console.error('Failed to fetch media estimate:', e);
    mediaEstimate = null;
  } finally {
    mediaEstimateLoading = false;
  }
}

function scheduleFetchMediaEstimate() {
  if (photoDebounceTimer) clearTimeout(photoDebounceTimer);
  photoDebounceTimer = setTimeout(() => {
    fetchMediaEstimate();
  }, DEBOUNCE_MS);
}

async function parseUrl() {
  if (!urlInput.trim()) {
    effectiveParams = '';
    urlParseError = false;
    return;
  }
  try {
    const result = await invoke<{ effective_params: string }>(
      'parse_inat_url',
      { url: urlInput },
    );
    effectiveParams = result.effective_params;
    urlParseError = false;
  } catch (e) {
    console.error('Failed to parse URL:', e);
    urlParseError = true;
    effectiveParams = '';
  }
}

function calculateEstimatedSize(): number | null {
  if (observationCount === null) return null;
  let sizeBytes = observationCount * BYTES_PER_OBSERVATION;
  if (includeSimpleMultimedia)
    sizeBytes += observationCount * BYTES_PER_OBSERVATION_MULTIMEDIA;
  if (includeIdentifications)
    sizeBytes += observationCount * BYTES_PER_OBSERVATION_IDENTIFICATIONS;
  if (includeComments)
    sizeBytes += observationCount * BYTES_PER_OBSERVATION_COMMENTS;
  if (fetchMedia && mediaEstimate && mediaEstimate.sample_size > 0) {
    const photosPerObs = mediaEstimate.photo_count / mediaEstimate.sample_size;
    const soundsPerObs = mediaEstimate.sound_count / mediaEstimate.sample_size;
    sizeBytes += Math.round(photosPerObs * observationCount) * BYTES_PER_PHOTO;
    sizeBytes += Math.round(soundsPerObs * observationCount) * BYTES_PER_SOUND;
  }
  return sizeBytes;
}

function buildGenerateParams(outputPath: string): GenerateParams {
  const extensions: string[] = [];
  if (includeSimpleMultimedia) extensions.push('SimpleMultimedia');
  if (includeAudiovisual) extensions.push('Audiovisual');
  if (includeIdentifications) extensions.push('Identifications');
  if (includeComments) extensions.push('Comments');

  return filterMode === 'url'
    ? {
        output_path: outputPath,
        taxon_id: null,
        place_id: null,
        user: null,
        d1: null,
        d2: null,
        created_d1: null,
        created_d2: null,
        url_params: effectiveParams || null,
        fetch_media: fetchMedia,
        extensions,
      }
    : {
        output_path: outputPath,
        taxon_id: taxonId,
        place_id: placeId,
        user: userId ? userId.toString() : null,
        d1: observedDateRange === 'custom' && observedD1 ? observedD1 : null,
        d2: observedDateRange === 'custom' && observedD2 ? observedD2 : null,
        created_d1:
          createdDateRange === 'custom' && createdD1 ? createdD1 : null,
        created_d2:
          createdDateRange === 'custom' && createdD2 ? createdD2 : null,
        url_params: null,
        fetch_media: fetchMedia,
        extensions,
      };
}

async function handleDownload() {
  const filePath = await showSaveDialog({
    defaultPath: 'observations.zip',
    filters: [{ name: 'Darwin Core Archive', extensions: ['zip'] }],
  });
  if (!filePath) return;
  const resolvedPath = Array.isArray(filePath) ? filePath[0] : filePath;

  const estimatedSize = calculateEstimatedSize();
  if (estimatedSize !== null && estimatedSize > ONE_GB) {
    pendingDownloadPath = resolvedPath;
    showLargeDownloadDialog = true;
    return;
  }
  ondownloadstart(resolvedPath, buildGenerateParams(resolvedPath));
}

function handleConfirmLargeDownload() {
  showLargeDownloadDialog = false;
  if (pendingDownloadPath) {
    ondownloadstart(
      pendingDownloadPath,
      buildGenerateParams(pendingDownloadPath),
    );
    pendingDownloadPath = null;
  }
}

function handleCancelLargeDownload() {
  showLargeDownloadDialog = false;
  pendingDownloadPath = null;
}

// Trigger count fetch when filters change
$effect(() => {
  const deps = [
    filterMode,
    ...(filterMode === 'url'
      ? [effectiveParams]
      : [
          taxonId,
          placeId,
          userId,
          observedDateRange,
          observedD1,
          observedD2,
          createdDateRange,
          createdD1,
          createdD2,
        ]),
  ];
  void deps;
  observationCount = null;
  scheduleFetchCount();
});

// Trigger media estimate fetch when filters or fetchMedia change
$effect(() => {
  const deps = [
    filterMode,
    ...(filterMode === 'url'
      ? [effectiveParams]
      : [
          taxonId,
          placeId,
          userId,
          observedDateRange,
          observedD1,
          observedD2,
          createdDateRange,
          createdD1,
          createdD2,
        ]),
    fetchMedia,
  ];
  void deps;
  mediaEstimate = null;
  if (fetchMedia) scheduleFetchMediaEstimate();
});
</script>

<ol class="step-list ps-6">
  <li>
    <h2 class="h4 mb-3 flex items-center justify-between">
      <span>Filter observations</span>
      <SegmentedControl
        value={filterMode}
        onValueChange={(e) => { filterMode = (e.value || 'fields') as 'fields' | 'url'; }}
      >
        <SegmentedControl.Control class="border-0 bg-gray-200 p-0">
          <SegmentedControl.Indicator class="bg-gray-600" />
          <SegmentedControl.Item value="fields">
            <SegmentedControl.ItemText class="text-xs">Fields</SegmentedControl.ItemText>
            <SegmentedControl.ItemHiddenInput />
          </SegmentedControl.Item>
          <SegmentedControl.Item value="url">
            <SegmentedControl.ItemText class="text-xs">URL</SegmentedControl.ItemText>
            <SegmentedControl.ItemHiddenInput />
          </SegmentedControl.Item>
        </SegmentedControl.Control>
      </SegmentedControl>
    </h2>

    {#if filterMode === 'fields'}
      <div class="mb-6">
        <div class="space-y-4">
          <InatTaxonChooser bind:selectedId={taxonId} />
          <InatPlaceChooser bind:selectedId={placeId} />
          <InatUserChooser bind:selectedId={userId} />

          <div>
            <div class="block text-sm font-medium mb-2">Observation Date Range</div>
            <div class="space-y-2">
              <label class="flex items-center w-fit">
                <input
                  type="radio"
                  name="observed-range"
                  value="all"
                  checked={observedDateRange === 'all'}
                  onchange={() => observedDateRange = 'all'}
                />
                <span class="ml-2">All time</span>
              </label>
              <label class="flex items-center w-fit">
                <input
                  type="radio"
                  name="observed-range"
                  value="custom"
                  checked={observedDateRange === 'custom'}
                  onchange={() => observedDateRange = 'custom'}
                />
                <span class="ml-2">Custom range</span>
              </label>
              {#if observedDateRange === 'custom'}
                <div class="ml-6 space-y-2">
                  <div>
                    <label for="observed-d1" class="block text-xs mb-1">From</label>
                    <input id="observed-d1" type="date" class="input" bind:value={observedD1} />
                  </div>
                  <div>
                    <label for="observed-d2" class="block text-xs mb-1">To</label>
                    <input id="observed-d2" type="date" class="input" bind:value={observedD2} />
                  </div>
                </div>
              {/if}
            </div>
          </div>

          <div>
            <div class="block text-sm font-medium mb-2">Created Date Range</div>
            <div class="space-y-2">
              <label class="flex items-center w-fit">
                <input
                  type="radio"
                  name="created-range"
                  value="all"
                  checked={createdDateRange === 'all'}
                  onchange={() => createdDateRange = 'all'}
                />
                <span class="ml-2">All time</span>
              </label>
              <label class="flex items-center w-fit">
                <input
                  type="radio"
                  name="created-range"
                  value="custom"
                  checked={createdDateRange === 'custom'}
                  onchange={() => createdDateRange = 'custom'}
                />
                <span class="ml-2">Custom range</span>
              </label>
              {#if createdDateRange === 'custom'}
                <div class="ml-6 space-y-2">
                  <div>
                    <label for="created-d1" class="block text-xs mb-1">From</label>
                    <input id="created-d1" type="date" class="input" bind:value={createdD1} />
                  </div>
                  <div>
                    <label for="created-d2" class="block text-xs mb-1">To</label>
                    <input id="created-d2" type="date" class="input" bind:value={createdD2} />
                  </div>
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {:else}
      <div class="mb-6 space-y-3">
        <label for="inat-url" class="block text-sm font-medium">
          Paste an iNaturalist observations URL
        </label>
        <input
          id="inat-url"
          type="text"
          class="input w-full"
          placeholder="E.g. https://www.inaturalist.org/observations?taxon_id=47790"
          bind:value={urlInput}
          onblur={parseUrl}
          onkeydown={(e) => { if (e.key === 'Enter') parseUrl(); }}
        />
        {#if urlParseError}
          <p class="text-red-600 text-sm">Could not parse URL</p>
        {:else if effectiveParams}
          <div class="text-sm text-gray-600">
            <span class="font-medium">Effective params:</span>
            <code class="ml-1 break-all">{effectiveParams}</code>
          </div>
        {:else if urlInput}
          <p class="text-gray-500 text-sm">No recognized parameters found</p>
        {/if}
      </div>
    {/if}
  </li>

  <li>
    <h2 class="h4 mb-3">Choose content</h2>

    <div class="mb-6">
      <div class="space-y-2">
        <label class="flex items-start w-fit space-x-2">
          <input name="fetchMedia" class="checkbox mt-1" type="checkbox" bind:checked={fetchMedia} />
          <div>
            <p>Download photos &amp; sounds</p>
            <p class="text-gray-500">
              Include all observation photos and sounds in the archive itself for backup or offline use
            </p>
          </div>
        </label>

        <div class="mt-3">
          <h3 class="h6">Extensions</h3>
          <p class="mb-4 text-gray-500">Files that contain extra data associated with occurrences.</p>
          <div class="ml-4 space-y-2">
            <ExtensionCheckbox
              bind:value={includeSimpleMultimedia}
              name="simpleMultimedia"
              title="Simple Multimedia"
              desc="Photo and sound data with attribution"
              url="https://rs.gbif.org/extension/gbif/1.0/multimedia.xml"
            />
            <!-- Audiovisual doesn't add much to SimpleMultimedia, let's see if we can do without it -->
            <!-- <ExtensionCheckbox
              bind:value={includeAudiovisual}
              name="audiovisual"
              title="Audiovisual Media Description"
              desc="Photo and sound data with attribution, taxonomic, and geographic metadata"
              url="https://rs.gbif.org/extension/ac/audiovisual_2024_11_07.xml"
            /> -->
            <ExtensionCheckbox
              bind:value={includeIdentifications}
              name="identifications"
              title="Identification History"
              desc="All identifications associated with the observation"
              url="https://rs.gbif.org/extension/dwc/identification_history_2025-07-10.xml"
            />
            <ExtensionCheckbox
              bind:value={includeComments}
              name="comments"
              title="Comments"
              desc="Discussion comments associated with the observation"
              url="https://schema.org/Comment"
            />
          </div>
        </div>
      </div>
    </div>
  </li>
</ol>

<div class="mb-6 p-4 border rounded">
  {#if countLoading}
    <div class="text-gray-600">Loading...</div>
  {:else if countError}
    <div class="text-red-600">Unable to load observation count</div>
  {:else if observationCount !== null}
    <div>{observationCount.toLocaleString()} observations match</div>
    {#if mediaEstimateLoading}
      <div class="text-gray-500 text-sm mt-1">Estimating size...</div>
    {:else}
      {@const estimatedSize = calculateEstimatedSize()}
      {#if estimatedSize !== null}
        <div class="text-gray-600 text-sm mt-1">
          Estimated archive size: {formatBytes(estimatedSize)}
        </div>
      {/if}
    {/if}
  {:else}
    <div class="text-gray-500">Enter filters to see observation count</div>
  {/if}
</div>

<button
  type="button"
  class="btn preset-filled w-full"
  disabled={countLoading || countError !== null || !observationCount}
  onclick={handleDownload}
>
  Download Archive
</button>

{#if showLargeDownloadDialog}
  {@const estimatedSize = calculateEstimatedSize()}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
      <h2 class="text-xl font-bold mb-4">Large Download Warning</h2>
      <p class="mb-6">
        The estimated archive size is ~{estimatedSize ? formatBytes(estimatedSize) : 'unknown'}.
        Are you sure you want to continue?
      </p>
      <div class="flex gap-3">
        <button type="button" class="btn preset-tonal flex-1" onclick={handleCancelLargeDownload}>
          Cancel
        </button>
        <button type="button" class="btn preset-filled flex-1" onclick={handleConfirmLargeDownload}>
          Download Anyway
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  ol.step-list {
    list-style: none;
    counter-reset: step;
  }
  ol.step-list > li {
    counter-increment: step;
  }
  ol.step-list > li > h2 {
    position: relative;
    display: flex;
    flex-direction: row;
    align-items: center;
  }
  ol.step-list > li > h2::before {
    content: counter(step);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5em;
    height: 1.5em;
    border-radius: 9999px;
    background: currentColor;
    color: white;
    background-color: black;
    line-height: 0;
    font-size: 0.875rem;
    margin-right: 0.5rem;
    position: absolute;
    left: -2em;
  }
</style>
