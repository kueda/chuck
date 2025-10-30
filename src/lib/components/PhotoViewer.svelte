<script lang="ts">
  import { Dialog, Portal } from '@skeletonlabs/skeleton-svelte';
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { convertFileSrc } from '@tauri-apps/api/core';

  const DEFAULT_ZOOM = 0.5;

  interface Props {
    open: boolean;
    photos: string[];
    initialIndex?: number;
  }

  let {
    open = $bindable(false),
    photos,
    initialIndex = 0
  }: Props = $props();

  let currentIndex = $state(initialIndex);
  let zoom = $state(DEFAULT_ZOOM);
  let panX = $state(0);
  let panY = $state(0);
  let imgElement: HTMLImageElement | null = $state(null);
  let containerElement: HTMLDivElement | null = $state(null);
  let convertedPhotoUrl = $state<string | null>(null);

  // Check if a path is a local file path (not a URL)
  function isLocalPath(path: string): boolean {
    return !path.startsWith('http://') && !path.startsWith('https://');
  }

  // Computed current photo URL (raw from photos array)
  const currentPhotoUrl = $derived(photos[currentIndex] || null);

  // Convert local paths to asset URLs when currentPhotoUrl changes
  $effect(() => {
    if (currentPhotoUrl && isLocalPath(currentPhotoUrl)) {
      (async () => {
        try {
          const cachedPath = await invoke<string>('get_photo', { photoPath: currentPhotoUrl });
          convertedPhotoUrl = convertFileSrc(cachedPath);
        } catch (error) {
          console.error('Failed to load local photo:', currentPhotoUrl, error);
          convertedPhotoUrl = null;
        }
      })();
    } else {
      convertedPhotoUrl = currentPhotoUrl;
    }
  });

  // Use converted URL for display
  const displayPhotoUrl = $derived(convertedPhotoUrl);

  const isPannable = $derived(() => {
    if (!imgElement || !containerElement) return false;

    const imgRect = imgElement.getBoundingClientRect();
    const containerRect = containerElement.getBoundingClientRect();

    // Image is pannable if it's larger than container in either dimension
    return imgRect.width > containerRect.width || imgRect.height > containerRect.height;
  });

  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let panStartX = $state(0);
  let panStartY = $state(0);
  let hasDragged = $state(false);

  // Reset zoom and index when photos change or dialog opens
  $effect(() => {
    if (open && photos.length > 0) {
      currentIndex = Math.min(initialIndex, photos.length - 1);
      zoom = DEFAULT_ZOOM;
    }
  });

  // Reset zoom and pan when displayPhotoUrl changes
  $effect(() => {
    if (displayPhotoUrl) {
      zoom = DEFAULT_ZOOM;
      panX = 0;
      panY = 0;
    }
  });

  // Clamp pan to bounds when zoom changes
  $effect(() => {
    if (imgElement && containerElement && zoom) {
      // Get image and container dimensions
      const imgRect = imgElement.getBoundingClientRect();
      const containerRect = containerElement.getBoundingClientRect();

      // Calculate max pan based on half the excess of scaled image over container
      const maxPanX = Math.max(0, (imgRect.width - containerRect.width) / 2);
      const maxPanY = Math.max(0, (imgRect.height - containerRect.height) / 2);

      panX = Math.max(-maxPanX, Math.min(maxPanX, panX));
      panY = Math.max(-maxPanY, Math.min(maxPanY, panY));
    }
  });

  function handleWheel(e: WheelEvent) {
    e.preventDefault();

    if (!imgElement || !containerElement) return;

    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    const oldZoom = zoom;
    const newZoom = Math.max(0.5, Math.min(5, zoom * delta));

    if (newZoom === oldZoom) return; // No change

    // Calculate mouse position relative to container
    const containerRect = containerElement.getBoundingClientRect();
    const mouseX = e.clientX - containerRect.left - containerRect.width / 2;
    const mouseY = e.clientY - containerRect.top - containerRect.height / 2;

    // Adjust pan to keep the point under the mouse fixed
    // The point under the mouse should stay in the same place in viewport coordinates
    const zoomRatio = newZoom / oldZoom;
    panX = mouseX - zoomRatio * (mouseX - panX);
    panY = mouseY - zoomRatio * (mouseY - panY);

    zoom = newZoom;
  }

  function navigateNext() {
    if (photos.length === 0) return;
    currentIndex = (currentIndex + 1) % photos.length;
  }

  function navigatePrevious() {
    if (photos.length === 0) return;
    currentIndex = (currentIndex - 1 + photos.length) % photos.length;
  }

  function handleMouseDown(e: MouseEvent) {
    if (!isPannable()) return;

    isDragging = true;
    hasDragged = false;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    panStartX = panX;
    panStartY = panY;
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;

    const deltaX = e.clientX - dragStartX;
    const deltaY = e.clientY - dragStartY;

    // Check if we've moved more than 5px threshold
    if (!hasDragged && (Math.abs(deltaX) > 5 || Math.abs(deltaY) > 5)) {
      hasDragged = true;
    }

    if (hasDragged && imgElement && containerElement) {
      // Get image and container dimensions
      const imgRect = imgElement.getBoundingClientRect();
      const containerRect = containerElement.getBoundingClientRect();

      // With translate() scale() transform order:
      // - The image is first translated, then scaled from its center
      // - Max pan is when the image edge is at the viewport center
      // Calculate max pan based on half the excess of scaled image over container
      const maxPanX = Math.max(0, (imgRect.width - containerRect.width) / 2);
      const maxPanY = Math.max(0, (imgRect.height - containerRect.height) / 2);

      // Apply pan with bounds
      const newPanX = panStartX + deltaX;
      const newPanY = panStartY + deltaY;

      panX = Math.max(-maxPanX, Math.min(maxPanX, newPanX));
      panY = Math.max(-maxPanY, Math.min(maxPanY, newPanY));
    }
  }

  function handleMouseUp(e: MouseEvent) {
    if (!isDragging) return;

    const didDrag = hasDragged;
    isDragging = false;
    hasDragged = false;

    // If we didn't drag (just clicked) and image is fully visible, zoom to max
    if (!didDrag && !isPannable() && zoom < 5 && imgElement && containerElement) {
      // Calculate click position relative to image center
      const imgRect = imgElement.getBoundingClientRect();
      const containerRect = containerElement.getBoundingClientRect();

      // Click position in viewport
      const clickX = e.clientX;
      const clickY = e.clientY;

      // Calculate where this point is in the image (before zoom)
      const imgCenterX = imgRect.left + imgRect.width / 2;
      const imgCenterY = imgRect.top + imgRect.height / 2;

      // Offset from center before zoom
      const offsetX = clickX - imgCenterX;
      const offsetY = clickY - imgCenterY;

      // Zoom to max
      const oldZoom = zoom;
      zoom = 5;

      // Calculate new pan to keep the clicked point in the same place
      // After zoom, the image will be larger, so we need to adjust pan
      const zoomRatio = zoom / oldZoom;
      panX = -offsetX * (zoomRatio - 1);
      panY = -offsetY * (zoomRatio - 1);
    }
  }
</script>

<Dialog {open} closeOnInteractOutside={true} onOpenChange={(details) => { open = details.open; }}>
  <Portal>
    <Dialog.Backdrop class="fixed inset-0 z-[60] bg-black/90" />
    <Dialog.Positioner class="fixed inset-0 z-[60] flex items-center justify-center p-8">
      <Dialog.Content
        class="relative w-full h-full flex items-center justify-center"
        tabindex={0}
        onkeydown={(e) => {
          if (e.key === 'ArrowLeft') {
            e.preventDefault();
            navigatePrevious();
          } else if (e.key === 'ArrowRight') {
            e.preventDefault();
            navigateNext();
          }
        }}
      >
        <Dialog.CloseTrigger
          class="absolute top-4 right-4 btn preset-filled-surface z-10"
        >
          Close
        </Dialog.CloseTrigger>

        {#if photos.length > 1}
          <button
            type="button"
            class="absolute left-4 top-1/2 -translate-y-1/2 btn preset-filled-surface z-10 w-12 h-12 p-0 flex items-center justify-center"
            onclick={navigatePrevious}
            aria-label="Previous photo"
          >
            <ChevronLeft size={24} />
          </button>

          <button
            type="button"
            class="absolute right-4 top-1/2 -translate-y-1/2 btn preset-filled-surface z-10 w-12 h-12 p-0 flex items-center justify-center"
            onclick={navigateNext}
            aria-label="Next photo"
          >
            <ChevronRight size={24} />
          </button>
        {/if}

        {#if displayPhotoUrl}
          <div
            bind:this={containerElement}
            class="overflow-auto w-full h-full flex items-center justify-center"
            onwheel={handleWheel}
            onmousedown={handleMouseDown}
            onmousemove={handleMouseMove}
            onmouseup={handleMouseUp}
            onmouseleave={handleMouseUp}
            role="img"
            aria-label="Full size photo"
          >
            <img
              bind:this={imgElement}
              src={displayPhotoUrl}
              alt="Full size"
              class="max-w-none"
              style="transform: translate({panX}px, {panY}px) scale({zoom}); cursor: {isDragging ? 'grabbing' : isPannable() ? 'grab' : zoom > 1 ? 'zoom-out' : 'zoom-in'};"
            />
          </div>
          <div class="absolute bottom-4 left-1/2 -translate-x-1/2 bg-black/50 text-white px-4 py-2 rounded">
            Zoom: {Math.round(zoom * 100)}% • Scroll to zoom
            {#if photos.length > 1}
              • {currentIndex + 1} / {photos.length}
            {/if}
          </div>
        {:else}
          <div class="text-white text-center">
            No photo available
          </div>
        {/if}
      </Dialog.Content>
    </Dialog.Positioner>
  </Portal>
</Dialog>
