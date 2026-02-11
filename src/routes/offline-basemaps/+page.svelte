<script lang="ts">
import { Progress } from '@skeletonlabs/skeleton-svelte';
import { Trash2 } from 'lucide-svelte';
import maplibregl from 'maplibre-gl';
import { onMount } from 'svelte';
import { buildMapStyle } from '$lib/mapStyle';
import {
  type BasemapInfo,
  type Bounds,
  deleteBasemap,
  downloadRegionalBasemap,
  estimateRegionalTiles,
  invoke,
  listBasemaps,
  listen,
} from '$lib/tauri-api';

import 'maplibre-gl/dist/maplibre-gl.css';

type Phase =
  | 'idle'
  | 'connecting'
  | 'downloading'
  | 'finalizing'
  | 'complete'
  | 'error';

// Global download state
let selectedZoom = $state(6);
let phase = $state<Phase>('idle');
let tilesDownloaded = $state(0);
let tilesTotal = $state(0);
let bytesDownloaded = $state(0);
let errorMessage = $state('');
let downloadTarget = $state<'global' | 'regional' | null>(null);

// Basemap list
let basemaps = $state<BasemapInfo[]>([]);

// Regional download state
let regionalZoom = $state(12);
let estimatedTiles = $state<number | null>(null);
let estimateTimer: ReturnType<typeof setTimeout> | null = null;

// Map state
let mapContainer = $state<HTMLDivElement>();
let map = $state<maplibregl.Map | null>(null);

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

const REGIONAL_ZOOM_OPTIONS = [7, 8, 9, 10, 11, 12, 13, 14, 15];

const downloading = $derived(
  phase === 'connecting' || phase === 'downloading' || phase === 'finalizing',
);

const progressPercent = $derived.by(() => {
  if (tilesTotal > 0) {
    return (tilesDownloaded / tilesTotal) * 100;
  }
  return undefined;
});

const hasGlobal = $derived(basemaps.some((b) => b.id === 'global'));

function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} bytes`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  if (bytes < 1_000_000_000) {
    return `${(bytes / 1_000_000).toFixed(1)} MB`;
  }
  return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
}

function formatBounds(bounds: Bounds): string {
  return (
    `${bounds.minLat.toFixed(1)}, ${bounds.minLon.toFixed(1)}` +
    ` to ${bounds.maxLat.toFixed(1)}, ${bounds.maxLon.toFixed(1)}`
  );
}

async function refreshBasemaps() {
  try {
    basemaps = await listBasemaps();
  } catch {
    basemaps = [];
  }
}

async function startGlobalDownload() {
  downloadTarget = 'global';
  phase = 'connecting';
  tilesDownloaded = 0;
  tilesTotal = 0;
  bytesDownloaded = 0;
  errorMessage = '';

  try {
    await invoke('download_basemap', { maxZoom: selectedZoom });
    phase = 'complete';
    await refreshBasemaps();
  } catch (e) {
    if (String(e).includes('cancelled')) {
      phase = 'idle';
    } else {
      phase = 'error';
      errorMessage = String(e);
    }
  }
  downloadTarget = null;
}

async function startRegionalDownload() {
  if (!map) return;
  const mapBounds = map.getBounds();
  const bounds: Bounds = {
    minLon: mapBounds.getWest(),
    minLat: mapBounds.getSouth(),
    maxLon: mapBounds.getEast(),
    maxLat: mapBounds.getNorth(),
  };

  downloadTarget = 'regional';
  phase = 'connecting';
  tilesDownloaded = 0;
  tilesTotal = 0;
  bytesDownloaded = 0;
  errorMessage = '';

  try {
    await downloadRegionalBasemap(bounds, regionalZoom);
    phase = 'complete';
    await refreshBasemaps();
    updateRegionalBoundsOverlay();
  } catch (e) {
    if (String(e).includes('cancelled')) {
      phase = 'idle';
    } else {
      phase = 'error';
      errorMessage = String(e);
    }
  }
  downloadTarget = null;
}

async function cancelDownload() {
  try {
    await invoke('cancel_basemap_download');
  } catch {
    // ignore
  }
}

async function handleDelete(id: string) {
  try {
    await deleteBasemap(id);
    await refreshBasemaps();
    updateRegionalBoundsOverlay();
  } catch (e) {
    errorMessage = String(e);
  }
}

function updateTileEstimate() {
  if (estimateTimer) clearTimeout(estimateTimer);
  estimateTimer = setTimeout(async () => {
    if (!map) return;
    const mapBounds = map.getBounds();
    const bounds: Bounds = {
      minLon: mapBounds.getWest(),
      minLat: mapBounds.getSouth(),
      maxLon: mapBounds.getEast(),
      maxLat: mapBounds.getNorth(),
    };
    try {
      const result = await estimateRegionalTiles(bounds, regionalZoom);
      estimatedTiles = result.tiles;
    } catch {
      estimatedTiles = null;
    }
  }, 300);
}

function updateRegionalBoundsOverlay() {
  if (!map) return;
  if (!map.getSource('regional-bounds')) return;

  const features = basemaps
    .filter((b): b is BasemapInfo & { bounds: Bounds } => b.bounds !== null)
    .map((b) => ({
      type: 'Feature' as const,
      properties: { name: b.name },
      geometry: {
        type: 'Polygon' as const,
        coordinates: [
          [
            [b.bounds.minLon, b.bounds.minLat],
            [b.bounds.maxLon, b.bounds.minLat],
            [b.bounds.maxLon, b.bounds.maxLat],
            [b.bounds.minLon, b.bounds.maxLat],
            [b.bounds.minLon, b.bounds.minLat],
          ],
        ],
      },
    }));

  (map.getSource('regional-bounds') as maplibregl.GeoJSONSource).setData({
    type: 'FeatureCollection',
    features,
  });
}

function initMap() {
  if (!mapContainer) return;
  destroyMap();

  map = new maplibregl.Map({
    container: mapContainer,
    style: buildMapStyle(false),
    center: [0, 20],
    zoom: 2,
  });

  map.addControl(new maplibregl.NavigationControl(), 'top-right');

  map.on('load', () => {
    if (!map) return;

    map.addSource('regional-bounds', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    });

    map.addLayer({
      id: 'regional-bounds-fill',
      type: 'fill',
      source: 'regional-bounds',
      paint: {
        'fill-color': '#3b82f6',
        'fill-opacity': 0.15,
      },
    });

    map.addLayer({
      id: 'regional-bounds-outline',
      type: 'line',
      source: 'regional-bounds',
      paint: {
        'line-color': '#3b82f6',
        'line-width': 2,
      },
    });

    updateRegionalBoundsOverlay();
    updateTileEstimate();
  });

  map.on('moveend', () => {
    updateTileEstimate();
  });
}

function destroyMap() {
  if (map) {
    map.remove();
    map = null;
  }
}

// Re-estimate when zoom changes
$effect(() => {
  // Trigger on regionalZoom change
  void regionalZoom;
  if (map) updateTileEstimate();
});

onMount(() => {
  refreshBasemaps();
  initMap();

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
    destroyMap();
    unlistenPromise.then((fn) => fn());
    if (estimateTimer) clearTimeout(estimateTimer);
  };
});
</script>

<div class="p-6 max-w-2xl mx-auto">
  <h1 class="h3 mb-4">Offline Basemaps</h1>

  <!-- Downloaded basemaps list -->
  {#if basemaps.length > 0}
    <div class="mb-6">
      <h3 class="text-sm font-medium mb-2">
        Downloaded basemaps
      </h3>
      <div class="space-y-2">
        {#each basemaps as bm}
          <div
            class="flex items-center justify-between p-2
              bg-surface-200 dark:bg-surface-800
              rounded text-sm"
          >
            <div class="min-w-0 flex-1">
              <span class="font-medium">{bm.name}</span>
              <span class="text-surface-500 ml-2">
                zoom 0&ndash;{bm.maxZoom}
                &mdash; {formatBytes(bm.fileSize)}
              </span>
              {#if bm.bounds}
                <div class="text-xs text-surface-500">
                  {formatBounds(bm.bounds)}
                </div>
              {/if}
            </div>
            <button
              type="button"
              class="btn btn-sm preset-tonal ml-2
                flex-shrink-0"
              onclick={() => handleDelete(bm.id)}
              title="Delete"
            >
              <Trash2 size={14} />
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Global basemap section -->
  <div class="mb-6">
    <h3 class="text-sm font-medium mb-2">
      Global basemap
    </h3>
    <p class="text-xs text-surface-500 mb-2">
      Low-zoom tiles for the entire planet. Provides
      country-level context everywhere.
    </p>
    <div class="flex items-end gap-2">
      <label class="flex-1">
        <span class="text-xs">Max zoom</span>
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
      </label>
      <button
        type="button"
        class="btn preset-filled"
        onclick={startGlobalDownload}
      >
        {hasGlobal ? 'Re-download' : 'Download'}
      </button>
    </div>
  </div>

  <!-- Regional basemap section -->
  <div>
    <h3 class="text-sm font-medium mb-2">
      Regional basemap
    </h3>
    <p class="text-xs text-surface-500 mb-2">
      Pan and zoom the map to the area you want, then
      download high-zoom tiles for that region.
    </p>

    <!-- Embedded map -->
    <div
      bind:this={mapContainer}
      class="w-full h-[300px] rounded border mb-3"
    ></div>

    <div class="flex items-end gap-2">
      <label class="flex-none">
        <span class="text-xs">Max zoom</span>
        <select
          class="select mt-1"
          bind:value={regionalZoom}
        >
          {#each REGIONAL_ZOOM_OPTIONS as zoom}
            <option value={zoom}>Zoom {zoom}</option>
          {/each}
        </select>
      </label>
      <div class="flex-1 text-xs text-surface-500 pb-2">
        {#if estimatedTiles !== null}
          ~{estimatedTiles.toLocaleString()} tiles
        {/if}
      </div>
      <button
        type="button"
        class="btn preset-filled flex-none"
        onclick={startRegionalDownload}
        disabled={!map}
      >
        Download this area
      </button>
    </div>
  </div>
</div>

{#if downloading || phase === 'error'}
  <div
    class="fixed inset-0 bg-black/50 flex items-center
      justify-center z-50"
  >
    <div
      class="bg-surface-50 dark:bg-surface-900 rounded-lg
        p-6 max-w-md w-full mx-4 shadow-xl"
    >
      {#if downloading}
        <div class="text-sm mb-1 font-medium">
          {downloadTarget === 'regional'
            ? 'Regional download'
            : 'Global download'}
        </div>
        <div class="text-sm mb-3">
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
          class="w-full mb-4"
        >
          <Progress.Track>
            <Progress.Range />
          </Progress.Track>
        </Progress>
        <button
          type="button"
          class="btn preset-tonal w-full"
          onclick={cancelDownload}
        >
          Cancel
        </button>
      {:else}
        <div
          class="mb-4 p-3 bg-error-100 dark:bg-error-900
            rounded"
        >
          <p class="text-sm text-error-700 dark:text-error-300">
            {errorMessage}
          </p>
        </div>
        <button
          type="button"
          class="btn preset-filled w-full"
          onclick={() => {
            phase = 'idle';
          }}
        >
          Dismiss
        </button>
      {/if}
    </div>
  </div>
{/if}
