<script lang="ts">
  import OccurrenceDrawer from '$lib/components/OccurrenceDrawer.svelte';
  import MapBoundingBoxControl from '$lib/components/MapBoundingBoxControl.svelte';
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { onMount, onDestroy } from 'svelte';

  import type { SearchParams } from '$lib/utils/filterCategories';

  interface Props {
    coreIdColumn: string;
    params: SearchParams;
    center: [number, number];
    zoom: number;
    onMapMove: (center: [number, number], zoom: number) => void;
    onBoundsChange?: (bounds: { nelat: number; nelng: number; swlat: number; swlng: number } | null) => void;
  }

  const {
    coreIdColumn,
    params,
    center: initialCenter,
    zoom: initialZoom,
    onMapMove,
    onBoundsChange,
  }: Props = $props();


  let mapContainer: HTMLDivElement;
  let map = $state<maplibregl.Map | null>(null);
  let zoom = $state(initialZoom);
  let center: [number, number] = $state(initialCenter);

  let drawerOpen = $state(false);
  let selectedOccurrenceId = $state<string | number | null>(null);

  const currentBounds = $derived(
    params.nelat !== undefined && params.nelng !== undefined &&
    params.swlat !== undefined && params.swlng !== undefined
      ? {
          nelat: Number(params.nelat),
          nelng: Number(params.nelng),
          swlat: Number(params.swlat),
          swlng: Number(params.swlng)
        }
      : null
  );

  $effect(() => {
    if (!params || !map) return;

    map.removeLayer('occurrence-points');
    map.removeSource('occurrences');
    const urlSearchParams = new URLSearchParams(Object.entries(params));
    const tileUrl = `tiles://localhost/{z}/{x}/{y}?${urlSearchParams.toString()}`;
    map!.addSource('occurrences', {
      type: 'vector',
      tiles: [tileUrl],
      minzoom: 0,
      maxzoom: 14
    });
    map!.addLayer({
      id: 'occurrence-points',
      type: 'circle',
      source: 'occurrences',
      'source-layer': 'occurrences',
      paint: {
        'circle-radius': 3,
        'circle-color': '#3b82f6',
        'circle-stroke-color': '#ffffff',
        'circle-stroke-width': 1
      }
    });
  });

  onMount(() => {
    // Initialize MapLibre map
    map = new maplibregl.Map({
      container: mapContainer,
      style: {
        version: 8,
        sources: {
          osm: {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '© OpenStreetMap contributors'
          }
        },
        layers: [
          {
            id: 'osm',
            type: 'raster',
            source: 'osm',
            minzoom: 0,
            maxzoom: 19
          }
        ]
      },
      center: initialCenter,
      zoom: initialZoom
    });

    // Add navigation controls
    map.addControl(new maplibregl.NavigationControl({ showCompass: false }), 'top-left');

    // Wait for map to load before adding tile source
    map.on('load', () => {
      // Add vector tile source using Tauri custom protocol
      const urlSearchParams = new URLSearchParams(Object.entries(params));
      map!.addSource('occurrences', {
        type: 'vector',
        tiles: [`tiles://localhost/{z}/{x}/{y}?${urlSearchParams.toString()}`],
        minzoom: 0,
        maxzoom: 14
      });

      // Add point layer
      map!.addLayer({
        id: 'occurrence-points',
        type: 'circle',
        source: 'occurrences',
        'source-layer': 'occurrences',
        paint: {
          'circle-radius': 3,
          'circle-color': '#3b82f6',
          'circle-stroke-color': '#ffffff',
          'circle-stroke-width': 1
        }
      });

      // Handle marker clicks to open drawer
      map!.on('click', 'occurrence-points', (e) => {
        if (!e.features || e.features.length === 0) return;

        const feature = e.features[0];
        const coreId = feature.properties?.core_id;

        if (coreId) {
          console.log(`clicked occ ${coreId}`);
          selectedOccurrenceId = coreId;
          drawerOpen = true;
        }
      });

      // Change cursor on hover
      map!.on('mouseenter', 'occurrence-points', () => {
        if (map) map.getCanvas().style.cursor = 'pointer';
      });
      map!.on('mouseleave', 'occurrence-points', () => {
        if (map) map.getCanvas().style.cursor = '';
      });

      // Update zoom and center on move
      map!.on('move', () => {
        if (map) {
          zoom = map.getZoom();
          const c = map.getCenter();
          center = [c.lng, c.lat];
          onMapMove(center, zoom);
        }
      });
    });
  });

  function handleBoundsChange(bounds: { nelat: number; nelng: number; swlat: number; swlng: number }) {
    if (onBoundsChange) {
      onBoundsChange(bounds);
    }
  }

  function handleBoundsClear() {
    if (onBoundsChange) {
      onBoundsChange(null);
    }
  }

  onDestroy(() => {
    map?.remove();
  });
</script>

<div class="map-wrapper">
  <div bind:this={mapContainer} class="map-container"></div>
  <div class="debug-info">
    <div>Zoom: {zoom.toFixed(2)}</div>
    <div>Center: {center[0].toFixed(4)}, {center[1].toFixed(4)}</div>
  </div>

  {#if map}
    <div class="absolute top-4 right-4 z-10">
      <MapBoundingBoxControl
        {map}
        {currentBounds}
        onBoundsChange={handleBoundsChange}
        onClear={handleBoundsClear}
      />
    </div>
  {/if}
</div>

<OccurrenceDrawer
  bind:open={drawerOpen}
  occurrenceId={selectedOccurrenceId}
  {coreIdColumn}
  onClose={() => { drawerOpen = false; }}
/>

<style>
  .map-wrapper {
    width: 100%;
    height: 100%;
    position: relative;
  }

  .map-container {
    width: 100%;
    height: 100%;
  }

  .debug-info {
    position: absolute;
    bottom: 10px;
    left: 10px;
    background: rgba(255, 255, 255, 0.9);
    padding: 8px 12px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 12px;
    pointer-events: none;
    z-index: 1000;
  }
</style>
