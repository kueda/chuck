<script lang="ts">
import { onMount } from 'svelte';
import InatPlaceChooser from '$lib/components/InatPlaceChooser.svelte';
import InatProgressOverlay from '$lib/components/InatProgressOverlay.svelte';
import InatTaxonChooser from '$lib/components/InatTaxonChooser.svelte';
import InatUserChooser from '$lib/components/InatUserChooser.svelte';
import {
  getCurrentWindow,
  invoke,
  listen,
  showSaveDialog,
} from '$lib/tauri-api';
import ExtensionCheckbox from './ExtensionCheckbox.svelte';
import { formatETR } from './format-etr';

interface InatProgress {
  stage: 'fetching' | 'downloadingPhotos' | 'building' | 'complete' | 'error';
  current?: number;
  total?: number;
  message?: string;
}

// Keep in sync w/ src-tauri/commands/inat_download.rs
interface CountParams {
  taxon_id: number | null;
  place_id: number | null;
  user: string | null;
  d1: string | null;
  d2: string | null;
  created_d1: string | null;
  created_d2: string | null;
}

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
let fetchPhotos = $state<boolean>(false);
let includeSimpleMultimedia = $state<boolean>(true);
let includeAudiovisual = $state<boolean>(false);
let includeIdentifications = $state<boolean>(false);

let observationCount = $state<number | null>(null);
let countLoading = $state<boolean>(false);
let countError = $state<string | null>(null);

// Photo estimate state (for size calculation when fetchPhotos is checked)
interface PhotoEstimate {
  photo_count: number;
  sample_size: number;
}
let photoEstimate = $state<PhotoEstimate | null>(null);
let photoEstimateLoading = $state<boolean>(false);

// Size estimation constants (derived from sample archives)
const BYTES_PER_OBSERVATION = 500;
const BYTES_PER_PHOTO = 1_800_000;
const ONE_GB = 1_000_000_000;

// Auth state
interface AuthStatus {
  authenticated: boolean;
  username: string | null;
}

let authStatus = $state<AuthStatus>({ authenticated: false, username: null });
let authLoading = $state<boolean>(false);

// Progress state
let showProgress = $state<boolean>(false);
let progressStage = $state<'active' | 'building' | 'complete' | 'error'>(
  'active',
);
let observationsCurrent = $state<number | undefined>(undefined);
let observationsTotal = $state<number | undefined>(undefined);
let photosCurrent = $state<number | undefined>(undefined);
let photosTotal = $state<number | undefined>(undefined);
let progressMessage = $state<string | undefined>(undefined);

// ETR state
let downloadStartTime = $state<number | null>(null);
let lastETRUpdateTime = $state<number>(0);
let lastETRItemsDownloaded = $state<number>(0);
let smoothedRate = $state<number | null>(null);
let estimatedSecondsRemaining = $state<number | null>(null);
const ETR_UPDATE_INTERVAL_MS = 2000; // Update display every 2 seconds
const ETR_SMOOTHING_ALPHA = 0.3; // Weight for new rate samples (lower = smoother)

// Success dialog state
let showSuccessDialog = $state<boolean>(false);
let completedArchivePath = $state<string | null>(null);

// Large download confirmation dialog state
let showLargeDownloadDialog = $state<boolean>(false);
let pendingDownloadPath = $state<string | null>(null);

async function loadAuthStatus() {
  try {
    authStatus = await invoke<AuthStatus>('inat_get_auth_status');
  } catch (e) {
    console.error('Failed to load auth status:', e);
  }
}

async function handleSignIn() {
  authLoading = true;
  try {
    authStatus = await invoke<AuthStatus>('inat_authenticate');
  } catch (e) {
    console.error('Authentication failed:', e);
    alert(`Authentication failed: ${e}`);
  } finally {
    authLoading = false;
  }
}

async function handleSignOut() {
  authLoading = true;
  try {
    await invoke('inat_sign_out');
    authStatus = { authenticated: false, username: null };
  } catch (e) {
    console.error('Sign out failed:', e);
    alert(`Sign out failed: ${e}`);
  } finally {
    authLoading = false;
  }
}

function updateETR() {
  const now = Date.now();

  // Use photos for rate calculation if available (they dominate download time),
  // otherwise fall back to observations
  const hasPhotos = (photosTotal || 0) > 0;
  const itemsDownloaded = hasPhotos
    ? photosCurrent || 0
    : observationsCurrent || 0;
  const totalItems = hasPhotos ? photosTotal || 0 : observationsTotal || 0;

  // Initialize on first call
  if (!downloadStartTime) {
    downloadStartTime = now;
    lastETRUpdateTime = now;
    lastETRItemsDownloaded = itemsDownloaded;
    smoothedRate = null;
    estimatedSecondsRemaining = null;
    return;
  }

  // Only update display every 2 seconds
  const timeSinceLastUpdate = now - lastETRUpdateTime;
  if (timeSinceLastUpdate < ETR_UPDATE_INTERVAL_MS) {
    return;
  }

  // Need some progress to estimate
  if (itemsDownloaded === 0 || totalItems === 0) {
    estimatedSecondsRemaining = null;
    return;
  }

  // Calculate instantaneous rate from recent progress
  const itemsDelta = itemsDownloaded - lastETRItemsDownloaded;
  const secondsDelta = timeSinceLastUpdate / 1000;

  // If no progress in this window, keep previous estimate
  if (itemsDelta <= 0 || secondsDelta < 0.1) {
    lastETRUpdateTime = now;
    return;
  }

  const instantRate = itemsDelta / secondsDelta;

  // Apply exponential moving average to smooth rate fluctuations
  if (smoothedRate === null) {
    smoothedRate = instantRate;
  } else {
    smoothedRate =
      ETR_SMOOTHING_ALPHA * instantRate +
      (1 - ETR_SMOOTHING_ALPHA) * smoothedRate;
  }

  const remainingItems = totalItems - itemsDownloaded;

  // Avoid showing ETR if rate is too slow (might indicate stall)
  if (smoothedRate < 0.1) {
    // Less than 1 item per 10 seconds
    estimatedSecondsRemaining = null;
  } else {
    estimatedSecondsRemaining = remainingItems / smoothedRate;
  }

  lastETRUpdateTime = now;
  lastETRItemsDownloaded = itemsDownloaded;
}

// Debounce timer
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
const DEBOUNCE_MS = 500;

async function fetchCount() {
  countLoading = true;
  countError = null;

  try {
    const params: CountParams = {
      taxon_id: taxonId,
      place_id: placeId,
      user: userId ? userId.toString() : null,
      d1: observedDateRange === 'custom' && observedD1 ? observedD1 : null,
      d2: observedDateRange === 'custom' && observedD2 ? observedD2 : null,
      created_d1: createdDateRange === 'custom' && createdD1 ? createdD1 : null,
      created_d2: createdDateRange === 'custom' && createdD2 ? createdD2 : null,
    };

    const count = await invoke<number>('get_observation_count', { params });
    observationCount = count;
  } catch (e) {
    console.error('Failed to fetch count:', e);
    countError = 'Unable to load observation count';
    observationCount = null;
  } finally {
    countLoading = false;
  }
}

function scheduleFetchCount() {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }

  debounceTimer = setTimeout(() => {
    fetchCount();
  }, DEBOUNCE_MS);
}

// Photo estimate debounce timer
let photoDebounceTimer: ReturnType<typeof setTimeout> | null = null;

async function fetchPhotoEstimate() {
  if (!fetchPhotos) {
    photoEstimate = null;
    return;
  }

  photoEstimateLoading = true;

  try {
    const params: CountParams = {
      taxon_id: taxonId,
      place_id: placeId,
      user: userId ? userId.toString() : null,
      d1: observedDateRange === 'custom' && observedD1 ? observedD1 : null,
      d2: observedDateRange === 'custom' && observedD2 ? observedD2 : null,
      created_d1: createdDateRange === 'custom' && createdD1 ? createdD1 : null,
      created_d2: createdDateRange === 'custom' && createdD2 ? createdD2 : null,
    };

    photoEstimate = await invoke<PhotoEstimate>('estimate_photo_count', {
      params,
    });
  } catch (e) {
    console.error('Failed to fetch photo estimate:', e);
    photoEstimate = null;
  } finally {
    photoEstimateLoading = false;
  }
}

function scheduleFetchPhotoEstimate() {
  if (photoDebounceTimer) {
    clearTimeout(photoDebounceTimer);
  }

  photoDebounceTimer = setTimeout(() => {
    fetchPhotoEstimate();
  }, DEBOUNCE_MS);
}

function calculateEstimatedSize(): number | null {
  if (observationCount === null) return null;

  let sizeBytes = observationCount * BYTES_PER_OBSERVATION;

  if (fetchPhotos && photoEstimate && photoEstimate.sample_size > 0) {
    const photosPerObs = photoEstimate.photo_count / photoEstimate.sample_size;
    const estimatedPhotos = Math.round(photosPerObs * observationCount);
    sizeBytes += estimatedPhotos * BYTES_PER_PHOTO;
  }

  return sizeBytes;
}

function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} bytes`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  if (bytes < 1_000_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes < 1_000_000_000_000)
    return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  return `${(bytes / 1_000_000_000_000).toFixed(1)} TB`;
}

async function handleDownload() {
  // Open file picker
  const filePath = await showSaveDialog({
    defaultPath: 'observations.zip',
    filters: [
      {
        name: 'Darwin Core Archive',
        extensions: ['zip'],
      },
    ],
  });

  if (!filePath) {
    return; // User cancelled
  }

  const resolvedPath = Array.isArray(filePath) ? filePath[0] : filePath;

  // Check if estimated size exceeds 1GB
  const estimatedSize = calculateEstimatedSize();
  if (estimatedSize !== null && estimatedSize > ONE_GB) {
    pendingDownloadPath = resolvedPath;
    showLargeDownloadDialog = true;
    return;
  }

  // Proceed with download
  await startDownload(resolvedPath);
}

async function startDownload(filePath: string) {
  // Store path for later
  completedArchivePath = filePath;

  // Show progress overlay
  showProgress = true;
  progressStage = 'building';
  progressMessage = 'Starting...';

  // Reset ETR state
  downloadStartTime = null;
  lastETRUpdateTime = 0;
  lastETRItemsDownloaded = 0;
  smoothedRate = null;
  estimatedSecondsRemaining = null;

  try {
    // Build extensions array
    const extensions: string[] = [];
    if (includeSimpleMultimedia) extensions.push('SimpleMultimedia');
    if (includeAudiovisual) extensions.push('Audiovisual');
    if (includeIdentifications) extensions.push('Identifications');

    // Call generate command
    await invoke('generate_inat_archive', {
      params: {
        output_path: filePath,
        taxon_id: taxonId,
        place_id: placeId,
        user: userId ? userId.toString() : null,
        d1: observedDateRange === 'custom' && observedD1 ? observedD1 : null,
        d2: observedDateRange === 'custom' && observedD2 ? observedD2 : null,
        created_d1:
          createdDateRange === 'custom' && createdD1 ? createdD1 : null,
        created_d2:
          createdDateRange === 'custom' && createdD2 ? createdD2 : null,
        fetch_photos: fetchPhotos,
        extensions,
      },
    });

    // Success handled by progress event listener
  } catch (e) {
    console.error('Failed to generate archive:', e);
    progressStage = 'error';
    progressMessage = e instanceof Error ? e.message : String(e);
  }
}

function handleConfirmLargeDownload() {
  showLargeDownloadDialog = false;
  if (pendingDownloadPath) {
    startDownload(pendingDownloadPath);
    pendingDownloadPath = null;
  }
}

function handleCancelLargeDownload() {
  showLargeDownloadDialog = false;
  pendingDownloadPath = null;
}

function handleCancelDownload() {
  invoke('cancel_inat_archive').catch((e) => {
    console.error('Failed to cancel:', e);
  });
  showProgress = false;
}

async function handleOpenInChuck() {
  if (!completedArchivePath) return;

  try {
    await invoke('open_archive', { path: completedArchivePath });
    showSuccessDialog = false;

    // Close the download window
    getCurrentWindow().close();
  } catch (e) {
    console.error('Failed to open archive:', e);
    alert(`Failed to open archive: ${e}`);
  }
}

function handleCloseDialog() {
  showSuccessDialog = false;
}

onMount(() => {
  // Load auth status
  loadAuthStatus();

  // Listen for progress events
  const unlistenProgress = listen<InatProgress>('inat-progress', (event) => {
    const progress = event.payload;

    if (progress.stage === 'fetching') {
      progressStage = 'active';
      observationsCurrent = progress.current;
      observationsTotal = progress.total;
      updateETR();
    } else if (progress.stage === 'downloadingPhotos') {
      progressStage = 'active';
      photosCurrent = progress.current;
      photosTotal = progress.total;
      updateETR();
    } else if (progress.stage === 'building') {
      progressStage = 'building';
      progressMessage = progress.message;
    } else if (progress.stage === 'complete') {
      progressStage = 'complete';
      progressMessage = 'Archive created successfully!';

      setTimeout(() => {
        showProgress = false;
        showSuccessDialog = true;
      }, 1000);
    } else if (progress.stage === 'error') {
      progressStage = 'error';
      progressMessage = progress.message;
    }
  });

  return () => {
    unlistenProgress.then((fn) => fn());
  };
});

// Trigger count fetch when filters change
$effect(() => {
  // Track dependencies
  const deps = [
    taxonId,
    placeId,
    userId,
    observedDateRange,
    observedD1,
    observedD2,
    createdDateRange,
    createdD1,
    createdD2,
  ];

  // Clear previous count while debouncing
  observationCount = null;

  scheduleFetchCount();
});

// Trigger photo estimate fetch when filters change or fetchPhotos is toggled
$effect(() => {
  // Track dependencies
  const deps = [
    taxonId,
    placeId,
    userId,
    observedDateRange,
    observedD1,
    observedD2,
    createdDateRange,
    createdD1,
    createdD2,
    fetchPhotos,
  ];

  // Clear previous estimate while debouncing
  photoEstimate = null;

  if (fetchPhotos) {
    scheduleFetchPhotoEstimate();
  }
});
</script>

<div class="p-6 max-w-2xl mx-auto">
  <h1 class="h3 mb-3">Download from iNaturalist</h1>
  <div class="mb-6">
    <p>Download iNaturalist observations as a DarwinCore Archive you can open in Chuck.</p>
  </div>

  <!-- Auth Section -->
  <div class="mb-6 p-4 border rounded">
    {#if authStatus.authenticated}
      <div class="flex items-center justify-between">
        <div class="text-sm">
          {#if authStatus.username}
            <span class="text-green-600">Signed in as {authStatus.username}</span>
          {:else}
            <span class="text-green-600">Signed in</span>
          {/if}
        </div>
        <button
          type="button"
          class="btn preset-tonal-surface text-sm"
          disabled={authLoading}
          onclick={handleSignOut}
        >
          {authLoading ? 'Signing out...' : 'Sign Out'}
        </button>
      </div>
    {:else}
      <div class="flex items-center justify-between">
        <div class="text-sm text-gray-600">
          Sign in to access your private coordinates (optional)
        </div>
        <button
          type="button"
          class="btn preset-filled-surface text-sm"
          disabled={authLoading}
          onclick={handleSignIn}
        >
          {authLoading ? 'Signing in...' : 'Sign In'}
        </button>
      </div>
    {/if}
  </div>

  <div class="mb-6">
    <h2 class="h4 mb-3">Filters</h2>
    <div class="space-y-4">
      <InatTaxonChooser bind:selectedId={taxonId} />

      <InatPlaceChooser bind:selectedId={placeId} />

      <InatUserChooser bind:selectedId={userId} />

      <div>
        <div class="block text-sm font-medium mb-2">
          Observation Date Range
        </div>
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
                <input
                  id="observed-d1"
                  type="date"
                  class="input"
                  bind:value={observedD1}
                />
              </div>
              <div>
                <label for="observed-d2" class="block text-xs mb-1">To</label>
                <input
                  id="observed-d2"
                  type="date"
                  class="input"
                  bind:value={observedD2}
                />
              </div>
            </div>
          {/if}
        </div>
      </div>

      <div>
        <div class="block text-sm font-medium mb-2">
          Created Date Range
        </div>
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
                <input
                  id="created-d1"
                  type="date"
                  class="input"
                  bind:value={createdD1}
                />
              </div>
              <div>
                <label for="created-d2" class="block text-xs mb-1">To</label>
                <input
                  id="created-d2"
                  type="date"
                  class="input"
                  bind:value={createdD2}
                />
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <div class="mb-6">
    <h2 class="h5 mb-3">Download Options</h2>
    <div class="space-y-2">
      <label class="flex items-start w-fit space-x-2">
        <input name="fetchPhotos" class="checkbox mt-1" type="checkbox" bind:checked={fetchPhotos} />
        <div>
          <p>Download photos</p>
          <p class="text-gray-500">Include all observation photos in the archive itself for backup or offline use</p>
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
          <ExtensionCheckbox
            bind:value={includeAudiovisual}
            name="audiovisual"
            title="Audiovisual Media Description"
            desc="Photo and sound data with attribution, taxonomic, and geographic metadata"
            url="https://rs.gbif.org/extension/ac/audiovisual_2024_11_07.xml"
          />
          <ExtensionCheckbox
            bind:value={includeIdentifications}
            name="identifications"
            title="Identification History"
            desc="All identifications associated with the observation"
            url="https://rs.gbif.org/extension/dwc/identification_history_2025-07-10.xml"
         />
        </div>
      </div>
    </div>
  </div>

  <div class="mb-6 p-4 border rounded">
    {#if countLoading}
      <div class="text-gray-600">Loading...</div>
    {:else if countError}
      <div class="text-red-600">Unable to load observation count</div>
    {:else if observationCount !== null}
      <div>{observationCount.toLocaleString()} observations match</div>
      {#if photoEstimateLoading}
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
    disabled={countLoading || countError !== null || observationCount === null || observationCount === 0}
    onclick={handleDownload}
  >
    Download Archive
  </button>
</div>

{#if showProgress}
  <InatProgressOverlay
    stage={progressStage}
    observationsCurrent={observationsCurrent}
    observationsTotal={observationsTotal}
    photosCurrent={photosCurrent}
    photosTotal={photosTotal}
    message={progressMessage}
    estimatedTimeRemaining={formatETR(estimatedSecondsRemaining)}
    onCancel={handleCancelDownload}
  />
{/if}

{#if showSuccessDialog}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
      <h2 class="text-xl font-bold mb-4">Archive Created Successfully!</h2>
      <p class="mb-6">Your Darwin Core Archive has been created. Would you like to open it in Chuck?</p>

      <div class="flex gap-3">
        <button
          type="button"
          class="btn preset-filled flex-1"
          onclick={handleOpenInChuck}
        >
          Open in Chuck
        </button>
        <button
          type="button"
          class="btn preset-tonal flex-1"
          onclick={handleCloseDialog}
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}

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
        <button
          type="button"
          class="btn preset-tonal flex-1"
          onclick={handleCancelLargeDownload}
        >
          Cancel
        </button>
        <button
          type="button"
          class="btn preset-filled flex-1"
          onclick={handleConfirmLargeDownload}
        >
          Download Anyway
        </button>
      </div>
    </div>
  </div>
{/if}
