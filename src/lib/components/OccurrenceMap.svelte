<script lang="ts">
import maplibregl from 'maplibre-gl';
import { onDestroy, onMount } from 'svelte';
import 'maplibre-gl/dist/maplibre-gl.css';
import { buildMapStyle } from '$lib/mapStyle';
import { listBasemaps } from '$lib/tauri-api';

interface Props {
  latitude: number;
  longitude: number;
  uncertainty?: number;
}

const { latitude, longitude, uncertainty }: Props = $props();

let mapContainer: HTMLDivElement;
let map: maplibregl.Map | null = null;
let marker: maplibregl.Marker | null = null;

onMount(async () => {
  // Check if offline basemap is available
  let hasBasemap = false;
  try {
    const basemaps = await listBasemaps();
    hasBasemap = basemaps.length > 0;
  } catch {
    hasBasemap = false;
  }

  map = new maplibregl.Map({
    container: mapContainer,
    style: buildMapStyle(hasBasemap),
    center: [longitude, latitude],
    zoom: 12,
  });

  // Add navigation controls
  map.addControl(new maplibregl.NavigationControl(), 'top-right');

  // Add marker
  marker = new maplibregl.Marker({ color: '#ef4444' })
    .setLngLat([longitude, latitude])
    .addTo(map);

  // Add uncertainty circle if present
  if (uncertainty && uncertainty > 0) {
    map.on('load', () => {
      if (!map) return;

      // Create circle polygon from uncertainty radius
      const center = [longitude, latitude];
      const radiusInKm = uncertainty / 1000;
      const points = 64;
      const coords = [];

      for (let i = 0; i < points; i++) {
        const angle = (i * 360) / points;
        const dx = radiusInKm * Math.cos((angle * Math.PI) / 180);
        const dy = radiusInKm * Math.sin((angle * Math.PI) / 180);
        const lat = latitude + dy / 111.32;
        const lng =
          longitude + dx / (111.32 * Math.cos((latitude * Math.PI) / 180));
        coords.push([lng, lat]);
      }
      coords.push(coords[0]); // Close the polygon

      map.addSource('uncertainty-circle', {
        type: 'geojson',
        data: {
          type: 'Feature',
          geometry: {
            type: 'Polygon',
            coordinates: [coords],
          },
          properties: {},
        },
      });

      map.addLayer({
        id: 'uncertainty-fill',
        type: 'fill',
        source: 'uncertainty-circle',
        paint: {
          'fill-color': '#3b82f6',
          'fill-opacity': 0.2,
        },
      });

      map.addLayer({
        id: 'uncertainty-outline',
        type: 'line',
        source: 'uncertainty-circle',
        paint: {
          'line-color': '#3b82f6',
          'line-width': 2,
          'line-opacity': 0.5,
        },
      });
    });
  }
});

onDestroy(() => {
  if (map) {
    map.remove();
    map = null;
  }
});
</script>

<div bind:this={mapContainer} class="w-full h-[300px] rounded border"></div>
