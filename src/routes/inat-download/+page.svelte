<script lang="ts">
import { SegmentedControl, Tabs } from '@skeletonlabs/skeleton-svelte';
import { AlertCircle } from 'lucide-svelte';
import { onMount } from 'svelte';
import InatPlaceChooser from '$lib/components/InatPlaceChooser.svelte';
import InatProgressOverlay from '$lib/components/InatProgressOverlay.svelte';
import InatTaxonChooser from '$lib/components/InatTaxonChooser.svelte';
import InatUserChooser from '$lib/components/InatUserChooser.svelte';
import {
  getCurrentWindow,
  invoke,
  listen,
  showOpenDialog,
  showSaveDialog,
} from '$lib/tauri-api';
import ExtensionCheckbox from './ExtensionCheckbox.svelte';
import { formatETR } from './format-etr';

interface InatProgress {
  stage: 'fetching' | 'downloadingMedia' | 'building' | 'complete' | 'error';
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
  url_params: string | null;
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
let fetchMedia = $state<boolean>(false);
let includeSimpleMultimedia = $state<boolean>(true);
let includeAudiovisual = $state<boolean>(false);
let includeIdentifications = $state<boolean>(true);
let includeComments = $state<boolean>(true);

// Page mode: create a new archive or update an existing one
let pageMode = $state<'create' | 'update'>('create');

// Filter mode
let filterMode = $state<'fields' | 'url'>('fields');
let urlInput = $state('');
let effectiveParams = $state('');
let urlParseError = $state(false);

// Update mode state
interface ChuckArchiveInfo {
  inat_query: string | null;
  extensions: string[];
  has_media: boolean;
  file_size_bytes: number;
  pub_date: string | null;
}
let updateFilePath = $state<string | null>(null);
let updateArchiveInfo = $state<ChuckArchiveInfo | null>(null);
let updateArchiveError = $state<string | null>(null);
let updateObsCount = $state<number | null>(null);
let updateObsCountLoading = $state<boolean>(false);
let updateObsCountError = $state<string | null>(null);
let updateMediaEstimate = $state<MediaEstimate | null>(null);
let updateMediaEstimateLoading = $state<boolean>(false);

let observationCount = $state<number | null>(null);
let countLoading = $state<boolean>(false);
let countError = $state<string | null>(null);

// Media estimate state (for size calculation when fetchMedia is checked)
interface MediaEstimate {
  photo_count: number;
  sound_count: number;
  sample_size: number;
}
let mediaEstimate = $state<MediaEstimate | null>(null);
let mediaEstimateLoading = $state<boolean>(false);

// Size estimation constants (derived from sample archives with 518 observations)
// Measured compressed bytes per observation:
//   Base: ~79, SimpleMultimedia: ~16, Identifications: ~146
const SIZE_ESTIMATE_SAFETY_MARGIN = 1.2;
const BYTES_PER_OBSERVATION = 79 * SIZE_ESTIMATE_SAFETY_MARGIN;
const BYTES_PER_OBSERVATION_MULTIMEDIA = 16 * SIZE_ESTIMATE_SAFETY_MARGIN;
const BYTES_PER_OBSERVATION_IDENTIFICATIONS = 146 * SIZE_ESTIMATE_SAFETY_MARGIN;
const BYTES_PER_OBSERVATION_COMMENTS = 40 * SIZE_ESTIMATE_SAFETY_MARGIN;
const BYTES_PER_PHOTO = 1_800_000;
const BYTES_PER_SOUND = 1_000_000;
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
let mediaCurrent = $state<number | undefined>(undefined);
let mediaTotal = $state<number | undefined>(undefined);
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
let downloadCancelled = $state<boolean>(false);

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

  // Use media for rate calculation if available (they dominate download time),
  // otherwise fall back to observations
  const hasMedia = (mediaTotal || 0) > 0;
  const itemsDownloaded = hasMedia
    ? mediaCurrent || 0
    : observationsCurrent || 0;
  const totalItems = hasMedia ? mediaTotal || 0 : observationsTotal || 0;

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

  // Estimate exceeded or stalled: don't show ETR
  if (remainingItems <= 0) {
    estimatedSecondsRemaining = null;
  } else if (smoothedRate < 0.1) {
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
  // In URL mode with no effective params, don't fetch — no filters entered yet
  if (filterMode === 'url' && !effectiveParams) {
    observationCount = null;
    countLoading = false;
    countError = null;
    return;
  }
  countLoading = true;
  countError = null;

  try {
    const params: CountParams =
      filterMode === 'url'
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
            d1:
              observedDateRange === 'custom' && observedD1 ? observedD1 : null,
            d2:
              observedDateRange === 'custom' && observedD2 ? observedD2 : null,
            created_d1:
              createdDateRange === 'custom' && createdD1 ? createdD1 : null,
            created_d2:
              createdDateRange === 'custom' && createdD2 ? createdD2 : null,
            url_params: null,
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

async function parseUrl() {
  if (!urlInput.trim()) {
    effectiveParams = '';
    urlParseError = false;
    return;
  }
  try {
    const result = await invoke<{ effective_params: string }>(
      'parse_inat_url',
      {
        url: urlInput,
      },
    );
    effectiveParams = result.effective_params;
    urlParseError = false;
  } catch (e) {
    console.error('Failed to parse URL:', e);
    urlParseError = true;
    effectiveParams = '';
  }
}

// Photo estimate debounce timer
let photoDebounceTimer: ReturnType<typeof setTimeout> | null = null;

async function fetchMediaEstimate() {
  if (!fetchMedia) {
    mediaEstimate = null;
    return;
  }
  // In URL mode with no effective params, don't fetch
  if (filterMode === 'url' && !effectiveParams) {
    mediaEstimate = null;
    return;
  }

  mediaEstimateLoading = true;

  try {
    const params: CountParams =
      filterMode === 'url'
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
            d1:
              observedDateRange === 'custom' && observedD1 ? observedD1 : null,
            d2:
              observedDateRange === 'custom' && observedD2 ? observedD2 : null,
            created_d1:
              createdDateRange === 'custom' && createdD1 ? createdD1 : null,
            created_d2:
              createdDateRange === 'custom' && createdD2 ? createdD2 : null,
            url_params: null,
          };

    mediaEstimate = await invoke<MediaEstimate>('estimate_media_count', {
      params,
    });
  } catch (e) {
    console.error('Failed to fetch media estimate:', e);
    mediaEstimate = null;
  } finally {
    mediaEstimateLoading = false;
  }
}

function scheduleFetchMediaEstimate() {
  if (photoDebounceTimer) {
    clearTimeout(photoDebounceTimer);
  }

  photoDebounceTimer = setTimeout(() => {
    fetchMediaEstimate();
  }, DEBOUNCE_MS);
}

function calculateEstimatedSize(): number | null {
  if (observationCount === null) return null;

  let sizeBytes = observationCount * BYTES_PER_OBSERVATION;

  // Add extension costs
  if (includeSimpleMultimedia) {
    sizeBytes += observationCount * BYTES_PER_OBSERVATION_MULTIMEDIA;
  }
  if (includeIdentifications) {
    sizeBytes += observationCount * BYTES_PER_OBSERVATION_IDENTIFICATIONS;
  }
  if (includeComments) {
    sizeBytes += observationCount * BYTES_PER_OBSERVATION_COMMENTS;
  }

  // Add media costs (photos + sounds)
  if (fetchMedia && mediaEstimate && mediaEstimate.sample_size > 0) {
    const photosPerObs = mediaEstimate.photo_count / mediaEstimate.sample_size;
    const soundsPerObs = mediaEstimate.sound_count / mediaEstimate.sample_size;
    const estimatedPhotos = Math.round(photosPerObs * observationCount);
    const estimatedSounds = Math.round(soundsPerObs * observationCount);
    sizeBytes += estimatedPhotos * BYTES_PER_PHOTO;
    sizeBytes += estimatedSounds * BYTES_PER_SOUND;
  }

  return sizeBytes;
}

function calculateEstimatedUpdateSize(): number | null {
  if (updateObsCount === null || !updateArchiveInfo) return null;

  let sizeBytes = updateObsCount * BYTES_PER_OBSERVATION;

  if (updateArchiveInfo.extensions.includes('SimpleMultimedia')) {
    sizeBytes += updateObsCount * BYTES_PER_OBSERVATION_MULTIMEDIA;
  }
  if (updateArchiveInfo.extensions.includes('Identifications')) {
    sizeBytes += updateObsCount * BYTES_PER_OBSERVATION_IDENTIFICATIONS;
  }
  if (updateArchiveInfo.extensions.includes('Comments')) {
    sizeBytes += updateObsCount * BYTES_PER_OBSERVATION_COMMENTS;
  }

  if (
    updateArchiveInfo.has_media &&
    updateMediaEstimate &&
    updateMediaEstimate.sample_size > 0
  ) {
    const photosPerObs =
      updateMediaEstimate.photo_count / updateMediaEstimate.sample_size;
    const soundsPerObs =
      updateMediaEstimate.sound_count / updateMediaEstimate.sample_size;
    sizeBytes += Math.round(photosPerObs * updateObsCount) * BYTES_PER_PHOTO;
    sizeBytes += Math.round(soundsPerObs * updateObsCount) * BYTES_PER_SOUND;
  }

  return sizeBytes;
}

function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} bytes`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  if (bytes < 1_000_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes < 1_000_000_000_000)
    return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  return `${(bytes / 1_000_000_000_000).toFixed(1)} TB 😱`;
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
  downloadCancelled = false;

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
    if (includeComments) extensions.push('Comments');

    // Call generate command
    await invoke('generate_inat_archive', {
      params:
        filterMode === 'url'
          ? {
              output_path: filePath,
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
              output_path: filePath,
              taxon_id: taxonId,
              place_id: placeId,
              user: userId ? userId.toString() : null,
              d1:
                observedDateRange === 'custom' && observedD1
                  ? observedD1
                  : null,
              d2:
                observedDateRange === 'custom' && observedD2
                  ? observedD2
                  : null,
              created_d1:
                createdDateRange === 'custom' && createdD1 ? createdD1 : null,
              created_d2:
                createdDateRange === 'custom' && createdD2 ? createdD2 : null,
              url_params: null,
              fetch_media: fetchMedia,
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
  downloadCancelled = true;
  invoke('cancel_inat_archive').catch((e) => {
    console.error('Failed to cancel:', e);
  });
  showProgress = false;
}

async function handlePickUpdateFile() {
  const result = await showOpenDialog({
    filters: [{ name: 'Darwin Core Archive', extensions: ['zip'] }],
    multiple: false,
  });
  if (!result) return;
  const path = Array.isArray(result) ? result[0] : result;
  updateFilePath = path;
  updateArchiveInfo = null;
  updateArchiveError = null;
  updateObsCount = null;
  updateObsCountError = null;
  updateMediaEstimate = null;
  try {
    updateArchiveInfo = await invoke<ChuckArchiveInfo>(
      'read_chuck_archive_info',
      { path },
    );
  } catch (e) {
    updateArchiveError = e instanceof Error ? e.message : String(e);
    return;
  }

  if (updateArchiveInfo?.inat_query) {
    updateObsCountLoading = true;
    try {
      updateObsCount = await invoke<number>('get_update_observation_count', {
        path,
      });
    } catch (e) {
      updateObsCountError = e instanceof Error ? e.message : String(e);
    } finally {
      updateObsCountLoading = false;
    }

    if (updateArchiveInfo.has_media) {
      updateMediaEstimateLoading = true;
      try {
        updateMediaEstimate = await invoke<MediaEstimate>(
          'estimate_media_count',
          { params: { url_params: updateArchiveInfo.inat_query } },
        );
      } catch {
        // best-effort; size estimate will omit media if unavailable
      } finally {
        updateMediaEstimateLoading = false;
      }
    }
  }
}

async function handleUpdate() {
  if (!updateFilePath) return;
  completedArchivePath = updateFilePath;
  showProgress = true;
  progressStage = 'building';
  progressMessage = 'Starting...';
  downloadCancelled = false;
  downloadStartTime = null;
  lastETRUpdateTime = 0;
  lastETRItemsDownloaded = 0;
  smoothedRate = null;
  estimatedSecondsRemaining = null;
  try {
    await invoke('update_inat_archive', { path: updateFilePath });
  } catch (e) {
    console.error('Failed to update archive:', e);
    progressStage = 'error';
    progressMessage = e instanceof Error ? e.message : String(e);
  }
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
    if (downloadCancelled) return;
    const progress = event.payload;

    if (progress.stage === 'fetching') {
      progressStage = 'active';
      observationsCurrent = progress.current;
      observationsTotal = progress.total;
      updateETR();
    } else if (progress.stage === 'downloadingMedia') {
      progressStage = 'active';
      mediaCurrent = progress.current;
      mediaTotal = progress.total;
      updateETR();
    } else if (progress.stage === 'building') {
      progressStage = 'building';
      progressMessage = progress.message;
    } else if (progress.stage === 'complete') {
      progressStage = 'complete';
      progressMessage =
        pageMode === 'update'
          ? 'Archive updated successfully!'
          : 'Archive created successfully!';

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

  // Clear previous count while debouncing
  observationCount = null;

  scheduleFetchCount();
});

// Trigger media estimate fetch when filters change or fetchMedia is toggled
$effect(() => {
  // Track dependencies
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

  // Clear previous estimate while debouncing
  mediaEstimate = null;

  if (fetchMedia) {
    scheduleFetchMediaEstimate();
  }
});
</script>

<div class="p-6 max-w-4xl mx-auto">
  <h1 class="h3 mb-3">Download from iNaturalist</h1>
  <div class="mb-6">
    <p>Download iNaturalist observations as a DarwinCore Archive you can open in Chuck.</p>
  </div>

  <!-- Auth Section -->
  <div class="mb-6 p-4 border rounded">
    {#if authStatus.authenticated}
      <div class="flex items-center justify-between">
        <div class="text-sm">
          <p class="mb-3 text-green-600">
            {#if authStatus.username}
              Signed in as <strong>{authStatus.username}</strong>
            {:else}
              Signed in
            {/if}
          </p>
          <p>Private coordinates you can access will be included</p>
        </div>
        <button
          type="button"
          class="btn preset-tonal text-sm"
          disabled={authLoading}
          onclick={handleSignOut}
        >
          {authLoading ? 'Signing out...' : 'Sign Out'}
        </button>
      </div>
    {:else}
      <div class="flex items-center justify-between">
        <div class="text-sm text-gray-600">
          Sign in to access private coordinates (optional)
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

  <Tabs
    value={pageMode}
    onValueChange={(e) => { pageMode = (e.value || 'create') as 'create' | 'update'; }}
  >
    <Tabs.List class="mb-4">
      <Tabs.Trigger value="create">Create archive</Tabs.Trigger>
      <Tabs.Trigger value="update">Update existing</Tabs.Trigger>
      <Tabs.Indicator />
    </Tabs.List>

    <Tabs.Content value="create">
      <ol class="step-list ps-6">
        <li>
          <h2 class="h4 mb-3 flex items-center justify-between">
            <span>Filter observations</span>
            <SegmentedControl
              value={filterMode}
              onValueChange={(e) => { filterMode = (e.value || 'fields') as 'fields' | 'url'; }}
            >
              <SegmentedControl.Control class="border-0 bg-gray-200 p-0">
                <SegmentedControl.Indicator />
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
    </Tabs.Content>

    <Tabs.Content value="update">
      <div class="mb-6 space-y-4">
        <div>
          <p class="text-sm mb-3">
            Choose an existing Chuck archive to update with recently changed observations.
          </p>
          <button type="button" class="btn preset-tonal text-sm" onclick={handlePickUpdateFile}>
            {updateFilePath ? 'Choose different file' : 'Choose archive…'}
          </button>
          {#if updateFilePath}
            <p class="text-sm mt-2 text-gray-600 break-all">{updateFilePath}</p>
          {/if}
        </div>

        {#if updateArchiveError}
          <div class="p-3 border border-red-300 rounded text-red-600 text-sm">
            {updateArchiveError}
          </div>
        {:else if updateArchiveInfo}
          <div class="p-3 border rounded space-y-2 text-sm">
            <div class="flex flex-row items-center gap-1">
              <span class="font-medium">Filters:</span>
              {#if updateArchiveInfo.inat_query}
                <code class="break-all text-xs">{updateArchiveInfo.inat_query}</code>
              {:else}
                <span class="badge preset-filled-error-400-600">
                  <AlertCircle size={16} />
                  <span>Archive does not specify filters</span>
                </span>
              {/if}
            </div>
            <div class="flex flex-row gap-1">
              <span class="font-medium">Extensions:</span>
              {#if updateArchiveInfo.extensions.length > 0}
                <span>{updateArchiveInfo.extensions.join(', ')}</span>
              {:else}
                <span class="text-gray-500">none</span>
              {/if}
            </div>
            <div class="flex flex-row gap-1">
              <span class="font-medium">Media included:</span>
              <span>{updateArchiveInfo.has_media ? 'Yes' : 'No'}</span>
            </div>
            <div class="flex flex-row gap-1">
              <span class="font-medium">Current size:</span>
              <span>{formatBytes(updateArchiveInfo.file_size_bytes)}</span>
            </div>
          </div>

          {#if updateArchiveInfo.inat_query}
            <div class="p-3 border rounded text-sm">
              {#if updateObsCountLoading}
                <div class="text-gray-600">Counting updated observations…</div>
              {:else if updateObsCountError}
                <div class="text-red-600">Unable to count updated observations</div>
              {:else if updateObsCount !== null}
                <div>{updateObsCount.toLocaleString()} observations to update</div>
                {#if updateObsCount > 0}
                  {#if updateMediaEstimateLoading}
                    <div class="text-gray-500 text-sm mt-1">Estimating additions…</div>
                  {:else}
                    {@const estimatedAdditions = calculateEstimatedUpdateSize()}
                    {#if estimatedAdditions !== null}
                      <div class="text-gray-600 text-sm mt-1">
                        Estimated additions: up to {formatBytes(estimatedAdditions)}
                      </div>
                    {/if}
                  {/if}
                {/if}
              {/if}
            </div>
          {/if}
        {/if}
      </div>

      <button
        type="button"
        class="btn preset-filled w-full"
        disabled={!updateFilePath || !!updateArchiveError || !updateArchiveInfo?.inat_query}
        onclick={handleUpdate}
      >
        Update Archive
      </button>
    </Tabs.Content>
  </Tabs>
</div>

{#if showProgress}
  <InatProgressOverlay
    stage={progressStage}
    observationsCurrent={observationsCurrent}
    observationsTotal={observationsTotal}
    mediaCurrent={mediaCurrent}
    mediaTotal={mediaTotal}
    mediaIsEstimate={true}
    message={progressMessage}
    estimatedTimeRemaining={formatETR(estimatedSecondsRemaining)}
    onCancel={handleCancelDownload}
  />
{/if}

{#if showSuccessDialog}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
      <h2 class="text-xl font-bold mb-4">{pageMode === 'update' ? 'Archive Updated Successfully!' : 'Archive Created Successfully!'}</h2>
      <p class="mb-6">{pageMode === 'update' ? 'Your Darwin Core Archive has been updated.' : 'Your Darwin Core Archive has been created.'} Would you like to open it in Chuck?</p>

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
