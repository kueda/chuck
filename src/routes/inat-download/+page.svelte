<script lang="ts">
import { Tabs } from '@skeletonlabs/skeleton-svelte';
import { onMount } from 'svelte';
import InatProgressOverlay from '$lib/components/InatProgressOverlay.svelte';
import {
  type AuthStatus,
  cancelInatArchive,
  type GenerateParams,
  generateInatArchive,
  getCurrentWindow,
  getInatAuthStatus,
  inatAuthenticate,
  inatSignOut,
  listen,
  openArchive,
  updateInatArchive,
} from '$lib/tauri-api';
import CreateArchiveTab from './CreateArchiveTab.svelte';
import { formatETR } from './format-etr';
import UpdateArchiveTab from './UpdateArchiveTab.svelte';

interface InatProgress {
  stage: 'fetching' | 'downloadingMedia' | 'building' | 'complete' | 'error';
  current?: number;
  total?: number;
  message?: string;
}

let pageMode = $state<'create' | 'update'>('create');

let authStatus = $state<AuthStatus>({ authenticated: false, username: null });
let authLoading = $state<boolean>(false);

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

let showSuccessDialog = $state<boolean>(false);
let completedArchivePath = $state<string | null>(null);
let downloadCancelled = $state<boolean>(false);

async function loadAuthStatus() {
  try {
    authStatus = await getInatAuthStatus();
  } catch (e) {
    console.error('Failed to load auth status:', e);
  }
}

async function handleSignIn() {
  authLoading = true;
  try {
    authStatus = await inatAuthenticate();
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
    await inatSignOut();
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
  if (timeSinceLastUpdate < ETR_UPDATE_INTERVAL_MS) return;

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
  smoothedRate =
    smoothedRate === null
      ? instantRate
      : ETR_SMOOTHING_ALPHA * instantRate +
        (1 - ETR_SMOOTHING_ALPHA) * smoothedRate;

  const remainingItems = totalItems - itemsDownloaded;

  // Estimate exceeded or stalled: don't show ETR
  estimatedSecondsRemaining =
    remainingItems <= 0 || smoothedRate < 0.1
      ? null
      : remainingItems / smoothedRate;

  lastETRUpdateTime = now;
  lastETRItemsDownloaded = itemsDownloaded;
}

function resetProgressState() {
  showProgress = true;
  progressStage = 'building';
  progressMessage = 'Starting...';
  downloadCancelled = false;
  downloadStartTime = null;
  lastETRUpdateTime = 0;
  lastETRItemsDownloaded = 0;
  smoothedRate = null;
  estimatedSecondsRemaining = null;
}

async function handleDownloadStart(filePath: string, params: GenerateParams) {
  completedArchivePath = filePath;
  resetProgressState();
  try {
    await generateInatArchive(params);
  } catch (e) {
    console.error('Failed to generate archive:', e);
    progressStage = 'error';
    progressMessage = e instanceof Error ? e.message : String(e);
  }
}

async function handleUpdateStart(path: string) {
  completedArchivePath = path;
  resetProgressState();
  try {
    await updateInatArchive(path);
  } catch (e) {
    console.error('Failed to update archive:', e);
    progressStage = 'error';
    progressMessage = e instanceof Error ? e.message : String(e);
  }
}

function handleCancelDownload() {
  downloadCancelled = true;
  cancelInatArchive().catch((e) => {
    console.error('Failed to cancel:', e);
  });
  showProgress = false;
}

async function handleOpenInChuck() {
  if (!completedArchivePath) return;
  try {
    await openArchive(completedArchivePath);
    showSuccessDialog = false;
    getCurrentWindow().close();
  } catch (e) {
    console.error('Failed to open archive:', e);
    alert(`Failed to open archive: ${e}`);
  }
}

onMount(() => {
  loadAuthStatus();

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
      <CreateArchiveTab ondownloadstart={handleDownloadStart} />
    </Tabs.Content>

    <Tabs.Content value="update">
      <UpdateArchiveTab onupdatestart={handleUpdateStart} />
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
      <h2 class="text-xl font-bold mb-4">
        {pageMode === 'update' ? 'Archive Updated Successfully!' : 'Archive Created Successfully!'}
      </h2>
      <p class="mb-6">
        {pageMode === 'update'
          ? 'Your Darwin Core Archive has been updated.'
          : 'Your Darwin Core Archive has been created.'}
        Would you like to open it in Chuck?
      </p>
      <div class="flex gap-3">
        <button type="button" class="btn preset-filled flex-1" onclick={handleOpenInChuck}>
          Open in Chuck
        </button>
        <button
          type="button"
          class="btn preset-tonal flex-1"
          onclick={() => { showSuccessDialog = false; }}
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
