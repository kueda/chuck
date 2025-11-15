<script lang="ts">
  import { onMount } from 'svelte';
  import type maplibregl from 'maplibre-gl';
  import { X, VectorSquare, Pencil, Move } from 'lucide-svelte';

  let {
    map,
    currentBounds,
    onBoundsChange,
    onClear
  }: {
    map?: maplibregl.Map | null;
    currentBounds: { nelat: number; nelng: number; swlat: number; swlng: number } | null;
    onBoundsChange: (bounds: {
      nelat: number;
      nelng: number;
      swlat: number;
      swlng: number;
    }) => void;
    onClear: () => void;
  } = $props();

  let isDrawingMode = $state(false);
  let isDrawing = $state(false);
  let drawStartPoint: maplibregl.LngLat | null = null;
  let isResizing = $state(false);
  let resizeCorner: 'ne' | 'nw' | 'se' | 'sw' | null = null;
  let resizeFixedCorner: maplibregl.LngLat | null = null;
  let isDragging = $state(false);
  let dragStartPoint: maplibregl.LngLat | null = null;
  let dragStartBounds: { nelat: number; nelng: number; swlat: number; swlng: number } | null = null;

  let mounted = $state(false);
  let handlersAttached = $state(false);

  onMount(() => {
    mounted = true;
    return () => {
      mounted = false;
      if (!map) return;
      map.off('mousedown', handleMouseDown);
      map.off('mousemove', handleMouseMove);
      map.off('mouseup', handleMouseUp);
    };
  });

  $effect(() => {
    if (!map || !mounted) return;

    function attachHandlers() {
      // Only initialize once
      if (!handlersAttached) {
        initializeBoundingBoxLayer();

        map?.on('mousedown', handleMouseDown);
        map?.on('mousemove', handleMouseMove);
        map?.on('mouseup', handleMouseUp);

        handlersAttached = true;

        // Initialize bounding box visualization if bounds exist
        if (currentBounds) {
          const source = map?.getSource('bounding-box') as maplibregl.GeoJSONSource;
          if (source) {
            const { nelat, nelng, swlat, swlng } = currentBounds;
            const bounds = [
              [swlng, swlat],
              [nelng, swlat],
              [nelng, nelat],
              [swlng, nelat],
              [swlng, swlat]
            ];
            source.setData({
              type: 'Feature',
              properties: {},
              geometry: {
                type: 'Polygon',
                coordinates: [bounds]
              }
            });
            // Update resize handles to show corners and move icon
            updateResizeHandles(nelat, nelng, swlat, swlng);
          }
        }
      }
    }

    if (map.loaded()) {
      attachHandlers();
    } else {
      map.on('load', attachHandlers);
    }
  });

  function initializeBoundingBoxLayer() {
    if (!map) return;

    if ( !map.getSource('bounding-box') ) {
      map.addSource('bounding-box', {
        type: 'geojson',
        data: {
          type: 'Feature',
          properties: {},
          geometry: {
            type: 'Polygon',
            coordinates: [[]]
          }
        }
      });
    }

    if (!map.getLayer('bounding-box-fill')) {
      map.addLayer({
        id: 'bounding-box-fill',
        type: 'fill',
        source: 'bounding-box',
        paint: {
          'fill-color': '#088',
          'fill-opacity': 0.1
        }
      });
    }

    if (!map.getLayer('bounding-box-outline')) {
      map.addLayer({
        id: 'bounding-box-outline',
        type: 'line',
        source: 'bounding-box',
        paint: {
          'line-color': '#088',
          'line-width': 2,
          'line-dasharray': [2, 2]
        }
      });
    }

    // Add resize handle source and layer
    if (!map.getSource('bbox-handles')) {
      map.addSource('bbox-handles', {
        type: 'geojson',
        data: {
          type: 'FeatureCollection',
          features: []
        }
      });
    }

    // Circle layer for corner resize handles (exclude move handle)
    if (!map.getLayer('bbox-handles')) {
      map.addLayer({
        id: 'bbox-handles',
        type: 'circle',
        source: 'bbox-handles',
        filter: ['!=', ['get', 'corner'], 'move'],
        paint: {
          'circle-radius': 6,
          'circle-color': '#088',
          'circle-stroke-width': 2,
          'circle-stroke-color': '#fff'
        }
      });
    }

    // Circle background for move handle
    if (!map.getLayer('bbox-move-handle-bg')) {
      map.addLayer({
        id: 'bbox-move-handle-bg',
        type: 'circle',
        source: 'bbox-handles',
        filter: ['==', ['get', 'corner'], 'move'],
        paint: {
          'circle-radius': 12,
          'circle-color': '#088',
          'circle-stroke-width': 2,
          'circle-stroke-color': '#fff'
        }
      });
    }

    // Add move icon layer with SVG
    if (!map.getLayer('bbox-move-handle-icon')) {
      // Create SVG icon for move handle
      const moveIconSvg = `
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M5 9l-3 3 3 3M9 5l3-3 3 3M15 19l-3 3-3-3M19 9l3 3-3 3M2 12h20M12 2v20"
                stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      `;
      const moveIconImage = new Image(16, 16);
      moveIconImage.onload = () => {
        if (!map.hasImage('move-icon')) {
          map.addImage('move-icon', moveIconImage);
        }
      };
      moveIconImage.src = 'data:image/svg+xml;base64,' + btoa(moveIconSvg);

      map.addLayer({
        id: 'bbox-move-handle-icon',
        type: 'symbol',
        source: 'bbox-handles',
        filter: ['==', ['get', 'corner'], 'move'],
        layout: {
          'icon-image': 'move-icon',
          'icon-size': 1,
          'icon-allow-overlap': true
        }
      });
    }
  }

  function updateResizeHandles(nelat: number, nelng: number, swlat: number, swlng: number) {
    if (!map) return;

    const centerLng = (nelng + swlng) / 2;

    const handleSource = map.getSource('bbox-handles') as maplibregl.GeoJSONSource;
    if (handleSource) {
      handleSource.setData({
        type: 'FeatureCollection',
        features: [
          { type: 'Feature', properties: { corner: 'ne' }, geometry: { type: 'Point', coordinates: [nelng, nelat] } },
          { type: 'Feature', properties: { corner: 'nw' }, geometry: { type: 'Point', coordinates: [swlng, nelat] } },
          { type: 'Feature', properties: { corner: 'se' }, geometry: { type: 'Point', coordinates: [nelng, swlat] } },
          { type: 'Feature', properties: { corner: 'sw' }, geometry: { type: 'Point', coordinates: [swlng, swlat] } },
          { type: 'Feature', properties: { corner: 'move' }, geometry: { type: 'Point', coordinates: [centerLng, swlat] } }
        ]
      });
    }
  }

  function clearResizeHandles() {
    if (!map) return;

    const handleSource = map.getSource('bbox-handles') as maplibregl.GeoJSONSource;
    if (handleSource) {
      handleSource.setData({
        type: 'FeatureCollection',
        features: []
      });
    }
  }

  function updateBoundingBoxLayer(start: maplibregl.LngLat, end: maplibregl.LngLat) {
    if (!map) return;

    const bounds = [
      [start.lng, start.lat],
      [end.lng, start.lat],
      [end.lng, end.lat],
      [start.lng, end.lat],
      [start.lng, start.lat]
    ];

    const source = map.getSource('bounding-box') as maplibregl.GeoJSONSource;

    if (source) {
      source.setData({
        type: 'Feature',
        properties: {},
        geometry: {
          type: 'Polygon',
          coordinates: [bounds]
        }
      });
    }

    // Update handles during drawing
    const nelat = Math.max(start.lat, end.lat);
    const swlat = Math.min(start.lat, end.lat);
    const nelng = Math.max(start.lng, end.lng);
    const swlng = Math.min(start.lng, end.lng);
    updateResizeHandles(nelat, nelng, swlat, swlng);
  }

  function clearBoundingBoxLayer() {
    if (!map) return;

    const source = map.getSource('bounding-box') as maplibregl.GeoJSONSource;
    if (source) {
      source.setData({
        type: 'Feature',
        properties: {},
        geometry: {
          type: 'Polygon',
          coordinates: [[]]
        }
      });
    }
    clearResizeHandles();
  }

  function toggleDrawingMode() {
    if (!map) return;

    isDrawingMode = !isDrawingMode;
    if (!isDrawingMode) {
      isDrawing = false;
      drawStartPoint = null;
    }

    const canvas = map.getCanvas();
    if (isDrawingMode) {
      canvas.classList.add('drawing');
      map.dragPan.disable();
      map.scrollZoom.disable();
      map.boxZoom.disable();
      map.doubleClickZoom.disable();
    } else {
      canvas.classList.remove('drawing');
      map.dragPan.enable();
      map.scrollZoom.enable();
      map.boxZoom.enable();
      map.doubleClickZoom.enable();
    }
  }

  function handleMouseDown(e: maplibregl.MapMouseEvent) {
    if (!map) return;

    // Check if clicking on a handle (check circle, background, and icon layers)
    const handleFeatures = map.queryRenderedFeatures(e.point, {
      layers: ['bbox-handles', 'bbox-move-handle-bg', 'bbox-move-handle-icon']
    });
    if (handleFeatures.length > 0 && currentBounds) {
      e.preventDefault();
      e.originalEvent.preventDefault();
      e.originalEvent.stopPropagation();

      const corner = handleFeatures[0].properties?.corner as 'ne' | 'nw' | 'se' | 'sw' | 'move';

      if (corner === 'move') {
        // Move handle - start dragging
        isDragging = true;
        dragStartPoint = e.lngLat;
        dragStartBounds = { ...currentBounds };
      } else {
        // Resize handle
        resizeCorner = corner;
        isResizing = true;

        // Set the fixed corner (opposite corner)
        const { nelat, nelng, swlat, swlng } = currentBounds;
        if (corner === 'ne') {
          resizeFixedCorner = { lng: swlng, lat: swlat } as maplibregl.LngLat;
        } else if (corner === 'nw') {
          resizeFixedCorner = { lng: nelng, lat: swlat } as maplibregl.LngLat;
        } else if (corner === 'se') {
          resizeFixedCorner = { lng: swlng, lat: nelat } as maplibregl.LngLat;
        } else if (corner === 'sw') {
          resizeFixedCorner = { lng: nelng, lat: nelat } as maplibregl.LngLat;
        }
      }

      // Disable map interactions while resizing/dragging
      map.dragPan.disable();
      map.scrollZoom.disable();
      map.boxZoom.disable();
      map.doubleClickZoom.disable();
      return;
    }

    // Normal drawing mode
    if (!isDrawingMode) return;

    e.preventDefault();
    e.originalEvent.preventDefault();
    e.originalEvent.stopPropagation();

    isDrawing = true;
    drawStartPoint = e.lngLat;
    clearBoundingBoxLayer();
  }

  function handleMouseMove(e: maplibregl.MapMouseEvent) {
    if (!map) return;

    // Update cursor based on what's under the mouse (check all handle layers)
    const handleFeatures = map.queryRenderedFeatures(e.point, {
      layers: ['bbox-handles', 'bbox-move-handle-bg', 'bbox-move-handle-icon']
    });

    if (handleFeatures.length > 0) {
      // Over a handle
      const corner = handleFeatures[0].properties?.corner;
      map.getCanvas().classList.remove('move', 'nesw-resize', 'nwse-resize');
      if (corner === 'move') {
        map.getCanvas().classList.add('move');
      } else if (corner === 'ne' || corner === 'sw') {
        map.getCanvas().classList.add('nesw-resize');
      } else if (corner === 'nw' || corner === 'se') {
        map.getCanvas().classList.add('nwse-resize');
      }
    } else if (!isDrawingMode && !isResizing && !isDragging) {
      // Not over anything interactive
      map.getCanvas().classList.remove('nesw-resize', 'nwse-resize', 'move');
    }

    if (isDrawingMode) {
      e.preventDefault();
      e.originalEvent.preventDefault();
      e.originalEvent.stopPropagation();
    }

    // Handle resizing
    if (isResizing && resizeFixedCorner) {
      e.preventDefault();
      e.originalEvent.preventDefault();
      e.originalEvent.stopPropagation();
      updateBoundingBoxLayer(resizeFixedCorner, e.lngLat);
      return;
    }

    // Handle dragging
    if (isDragging && dragStartPoint && dragStartBounds) {
      e.preventDefault();
      e.originalEvent.preventDefault();
      e.originalEvent.stopPropagation();

      const deltaLng = e.lngLat.lng - dragStartPoint.lng;
      const deltaLat = e.lngLat.lat - dragStartPoint.lat;

      const newBounds = {
        nelat: dragStartBounds.nelat + deltaLat,
        nelng: dragStartBounds.nelng + deltaLng,
        swlat: dragStartBounds.swlat + deltaLat,
        swlng: dragStartBounds.swlng + deltaLng
      };

      // Update visualization
      const start = { lng: newBounds.swlng, lat: newBounds.swlat } as maplibregl.LngLat;
      const end = { lng: newBounds.nelng, lat: newBounds.nelat } as maplibregl.LngLat;
      updateBoundingBoxLayer(start, end);
      return;
    }

    // Handle drawing
    if (!isDrawing || !drawStartPoint) return;
    updateBoundingBoxLayer(drawStartPoint, e.lngLat);
  }

  function handleMouseUp(e: maplibregl.MapMouseEvent) {
    if (!map) return;

    // Handle resize end
    if (isResizing && resizeFixedCorner) {
      e.preventDefault();
      e.originalEvent.preventDefault();
      e.originalEvent.stopPropagation();

      const endPoint = e.lngLat;
      const nelat = Math.max(resizeFixedCorner.lat, endPoint.lat);
      const swlat = Math.min(resizeFixedCorner.lat, endPoint.lat);
      const nelng = Math.max(resizeFixedCorner.lng, endPoint.lng);
      const swlng = Math.min(resizeFixedCorner.lng, endPoint.lng);

      const bounds = { nelat, nelng, swlat, swlng };
      onBoundsChange(bounds);

      isResizing = false;
      resizeCorner = null;
      resizeFixedCorner = null;

      // Re-enable map interactions
      map.dragPan.enable();
      map.scrollZoom.enable();
      map.boxZoom.enable();
      map.doubleClickZoom.enable();
      return;
    }

    // Handle drag end
    if (isDragging && dragStartPoint && dragStartBounds) {
      e.preventDefault();
      e.originalEvent.preventDefault();
      e.originalEvent.stopPropagation();

      const deltaLng = e.lngLat.lng - dragStartPoint.lng;
      const deltaLat = e.lngLat.lat - dragStartPoint.lat;

      const bounds = {
        nelat: dragStartBounds.nelat + deltaLat,
        nelng: dragStartBounds.nelng + deltaLng,
        swlat: dragStartBounds.swlat + deltaLat,
        swlng: dragStartBounds.swlng + deltaLng
      };

      onBoundsChange(bounds);

      isDragging = false;
      dragStartPoint = null;
      dragStartBounds = null;

      // Re-enable map interactions
      map.dragPan.enable();
      map.scrollZoom.enable();
      map.boxZoom.enable();
      map.doubleClickZoom.enable();
      return;
    }

    // Handle drawing end
    if (!isDrawingMode || !isDrawing || !drawStartPoint) return;

    e.preventDefault();
    e.originalEvent.preventDefault();
    e.originalEvent.stopPropagation();

    isDrawing = false;

    const endPoint = e.lngLat;

    // Calculate bounds
    const nelat = Math.max(drawStartPoint.lat, endPoint.lat);
    const swlat = Math.min(drawStartPoint.lat, endPoint.lat);
    const nelng = Math.max(drawStartPoint.lng, endPoint.lng);
    const swlng = Math.min(drawStartPoint.lng, endPoint.lng);

    const bounds = { nelat, nelng, swlat, swlng };
    onBoundsChange(bounds);

    // Exit drawing mode after drawing
    isDrawingMode = false;
    if (map) {
      map.getCanvas().classList.remove('drawing');
      // Re-enable map interactions
      map.dragPan.enable();
      map.scrollZoom.enable();
      map.boxZoom.enable();
      map.doubleClickZoom.enable();
    }
  }

  function handleClear() {
    clearBoundingBoxLayer();
    onClear();
  }

  $effect(() => {
    // Update bounding box visualization when bounds change
    // Explicitly track these dependencies
    const bounds = currentBounds;
    const attached = handlersAttached;

    if (!map || !map.loaded() || !attached) return;

    const source = map.getSource('bounding-box') as maplibregl.GeoJSONSource;
    if (!source) return;

    if (bounds) {
      const { nelat, nelng, swlat, swlng } = bounds;
      const boundsCoords = [
        [swlng, swlat],
        [nelng, swlat],
        [nelng, nelat],
        [swlng, nelat],
        [swlng, swlat]
      ];

      source.setData({
        type: 'Feature',
        properties: {},
        geometry: {
          type: 'Polygon',
          coordinates: [boundsCoords]
        }
      });

      // Update resize handles
      updateResizeHandles(nelat, nelng, swlat, swlng);
    } else {
      source.setData({
        type: 'Feature',
        properties: {},
        geometry: {
          type: 'Polygon',
          coordinates: [[]]
        }
      });
      clearResizeHandles();
    }
  });
</script>

<style>
  :global(.maplibregl-canvas.drawing) {
    cursor: crosshair !important;
  }
  :global(.maplibregl-canvas.nesw-resize) {
    cursor: nesw-resize !important;
  }
  :global(.maplibregl-canvas.nwse-resize) {
    cursor: nwse-resize !important;
  }
  :global(.maplibregl-canvas.move) {
    cursor: move !important;
  }
</style>

<div class="flex flex-col gap-2">
  <button
    type="button"
    class="btn {isDrawingMode ? 'preset-filled-surface-950-50' : 'preset-filled-surface-50-950'} shadow-lg relative"
    onclick={toggleDrawingMode}
    title={isDrawingMode ? 'Cancel drawing' : 'Draw bounding box'}
  >
    {#if isDrawingMode}
      <span class="badge-icon preset-filled-surface-50-950 absolute -right-2 -top-2 z-10 p-1">
        <Pencil size={10} />
      </span>
    {/if}
    <VectorSquare size={20} />
  </button>

  {#if currentBounds}
    <button
      type="button"
      class="btn preset-filled-secondary-50-950 shadow-lg"
      onclick={handleClear}
      title="Clear bounding box"
    >
      <X size={20} />
    </button>
  {/if}
</div>
