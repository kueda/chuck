<script lang="ts">
import { Dialog, Portal, Progress } from '@skeletonlabs/skeleton-svelte';
import { onMount } from 'svelte';
import { invoke, listen } from '$lib/tauri-api';

interface Props {
  open: boolean;
  onClose: () => void;
}

let { open = $bindable(), onClose }: Props = $props();

type Phase =
  | 'idle'
  | 'connecting'
  | 'downloading'
  | 'finalizing'
  | 'complete'
  | 'error';

let selectedZoom = $state(6);
let phase = $state<Phase>('idle');
let tilesDownloaded = $state(0);
let tilesTotal = $state(0);
let bytesDownloaded = $state(0);
let errorMessage = $state('');
let existingStatus = $state<{
  downloaded: boolean;
  maxZoom?: number;
  fileSize?: number;
} | null>(null);

const ZOOM_ESTIMATES: Record<number, string> = {
  3: '~2 MB',
  4: '~5 MB',
  5: '~17 MB',
  6: '~50 MB',
  7: '~120 MB',
  8: '~250 MB',
  9: '~600 MB',
  10: '~1.5 GB',
  11: '~3.5 GB',
  12: '~8 GB',
};

const downloading = $derived(
  phase === 'connecting' || phase === 'downloading' || phase === 'finalizing',
);

const progressPercent = $derived.by(() => {
  if (tilesTotal > 0) {
    return (tilesDownloaded / tilesTotal) * 100;
  }
  return undefined;
});

function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} bytes`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  if (bytes < 1_000_000_000) {
    return `${(bytes / 1_000_000).toFixed(1)} MB`;
  }
  return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
}

async function checkStatus() {
  try {
    existingStatus = await invoke<{
      downloaded: boolean;
      maxZoom?: number;
      fileSize?: number;
    }>('get_basemap_status');
  } catch {
    existingStatus = null;
  }
}

async function startDownload() {
  phase = 'connecting';
  tilesDownloaded = 0;
  tilesTotal = 0;
  bytesDownloaded = 0;
  errorMessage = '';

  try {
    await invoke('download_basemap', { maxZoom: selectedZoom });
    phase = 'complete';
    await checkStatus();
  } catch (e) {
    if (String(e).includes('cancelled')) {
      phase = 'idle';
    } else {
      phase = 'error';
      errorMessage = String(e);
    }
  }
}

async function cancelDownload() {
  try {
    await invoke('cancel_basemap_download');
  } catch {
    // ignore
  }
}

async function deleteBasemap() {
  try {
    await invoke('delete_basemap');
    await checkStatus();
  } catch (e) {
    errorMessage = String(e);
  }
}

onMount(() => {
  checkStatus();

  const unlistenPromise = listen<{
    tilesDownloaded: number;
    tilesTotal: number;
    bytesDownloaded: number;
    phase: string;
  }>('basemap-download-progress', (event) => {
    const p = event.payload;
    tilesDownloaded = p.tilesDownloaded;
    tilesTotal = p.tilesTotal;
    bytesDownloaded = p.bytesDownloaded;
    if (
      p.phase === 'connecting' ||
      p.phase === 'downloading' ||
      p.phase === 'finalizing' ||
      p.phase === 'complete'
    ) {
      phase = p.phase as Phase;
    }
  });

  return () => {
    unlistenPromise.then((fn) => fn());
  };
});
</script>

<Dialog
  {open}
  onOpenChange={(details) => {
    if (!details.open && !downloading) {
      open = false;
      onClose();
    }
  }}
>
  <Portal>
    <Dialog.Backdrop class="fixed inset-0 z-50 bg-black/50" />
    <Dialog.Positioner
      class="fixed inset-0 z-50 flex items-center justify-center p-4"
    >
      <Dialog.Content
        class="bg-surface-50 dark:bg-surface-900 rounded-lg p-6 max-w-md w-full shadow-xl"
      >
        <h2 class="text-xl font-bold mb-4">Offline Basemap</h2>

        {#if existingStatus?.downloaded}
          <div class="mb-4 p-3 bg-surface-200 dark:bg-surface-800 rounded">
            <p class="text-sm">
              Basemap downloaded
              {#if existingStatus.maxZoom !== undefined}
                (zoom 0&ndash;{existingStatus.maxZoom})
              {/if}
              {#if existingStatus.fileSize}
                &mdash; {formatBytes(existingStatus.fileSize)}
              {/if}
            </p>
          </div>
        {/if}

        {#if phase === 'idle' || phase === 'complete'}
          <p class="text-sm mb-4">
            Download a vector basemap for offline use. The map will
            render without an internet connection.
          </p>

          <label class="block mb-4">
            <span class="text-sm font-medium">Maximum zoom level</span>
            <select
              class="select mt-1 w-full"
              bind:value={selectedZoom}
            >
              {#each Object.entries(ZOOM_ESTIMATES) as [zoom, size]}
                <option value={Number(zoom)}>
                  Zoom {zoom} &mdash; {size}
                </option>
              {/each}
            </select>
            <span class="text-xs text-surface-500 mt-1 block">
              Higher zoom = more detail but larger download.
              Zoom 6 covers the whole planet at country level.
            </span>
          </label>

          <div class="flex gap-2">
            <button
              type="button"
              class="btn preset-filled flex-1"
              onclick={startDownload}
            >
              {existingStatus?.downloaded
                ? 'Re-download'
                : 'Download'}
            </button>
            {#if existingStatus?.downloaded}
              <button
                type="button"
                class="btn preset-tonal"
                onclick={deleteBasemap}
              >
                Delete
              </button>
            {/if}
            <button
              type="button"
              class="btn preset-tonal"
              onclick={() => {
                open = false;
                onClose();
              }}
            >
              {existingStatus?.downloaded ? 'Close' : 'Skip'}
            </button>
          </div>
        {:else if downloading}
          <div class="mb-4">
            <div class="text-sm mb-2">
              {#if phase === 'connecting'}
                Connecting to Protomaps...
              {:else if phase === 'downloading'}
                Downloading tiles...
                {tilesDownloaded.toLocaleString()}/{tilesTotal.toLocaleString()}
                ({formatBytes(bytesDownloaded)})
              {:else if phase === 'finalizing'}
                Finalizing basemap file...
              {/if}
            </div>
            <Progress
              value={phase === 'downloading'
                ? progressPercent
                : undefined}
              class="w-full"
            >
              <Progress.Track>
                <Progress.Range />
              </Progress.Track>
            </Progress>
          </div>
          <button
            type="button"
            class="btn preset-tonal w-full"
            onclick={cancelDownload}
          >
            Cancel
          </button>
        {:else if phase === 'error'}
          <div class="mb-4 p-3 bg-error-100 dark:bg-error-900 rounded">
            <p class="text-sm text-error-700 dark:text-error-300">
              {errorMessage}
            </p>
          </div>
          <div class="flex gap-2">
            <button
              type="button"
              class="btn preset-filled flex-1"
              onclick={startDownload}
            >
              Retry
            </button>
            <button
              type="button"
              class="btn preset-tonal"
              onclick={() => {
                phase = 'idle';
              }}
            >
              Back
            </button>
          </div>
        {/if}
      </Dialog.Content>
    </Dialog.Positioner>
  </Portal>
</Dialog>
