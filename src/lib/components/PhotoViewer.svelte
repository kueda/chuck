<script lang="ts">
import { Dialog, Portal } from '@skeletonlabs/skeleton-svelte';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import {
  ArrowDown,
  ArrowLeft as ArrowLeftIcon,
  ArrowRight as ArrowRightIcon,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  Maximize,
  X,
  ZoomIn,
  ZoomOut,
} from 'lucide-svelte';

const PAN_INCREMENT = 50; // pixels to pan with button click
const MAX_ZOOM = 5;
const FALLBACK_MIN_ZOOM = 0.5; // Fallback when image not loaded
const BTN_BASE_CLASSES = [
  'btn',
  'preset-filled-surface-900-100',
  'absolute',
  'z-10',
];
const NAV_BTN_CLASSES = [
  ...BTN_BASE_CLASSES,
  'top-1/2',
  '-translate-y-1/2',
  'w-12',
  'h-12',
  'p-0',
  'flex',
  'items-center',
  'justify-center',
];
const CONTROL_BTN_CLASSES = [
  'btn',
  'preset-filled-surface-900-100',
  'absolute',
  'z-10',
  'w-12',
  'h-12',
  'p-0',
  'flex',
  'items-center',
  'justify-center',
];

interface Props {
  open: boolean;
  photos: string[];
  initialIndex?: number;
}

let { open = $bindable(false), photos, initialIndex = 0 }: Props = $props();

let currentIndex = $state(initialIndex);
let zoom = $state(FALLBACK_MIN_ZOOM);
let panX = $state(0);
let panY = $state(0);
let imgElement: HTMLImageElement | null = $state(null);
let containerElement: HTMLDivElement | null = $state(null);
let convertedPhotoUrl = $state<string | null>(null);
let navPrevButton: HTMLButtonElement | null = $state(null);
let navNextButton: HTMLButtonElement | null = $state(null);
let panLeftButton: HTMLButtonElement | null = $state(null);
let panRightButton: HTMLButtonElement | null = $state(null);
let panUpButton: HTMLButtonElement | null = $state(null);
let panDownButton: HTMLButtonElement | null = $state(null);
let zoomInButton: HTMLButtonElement | null = $state(null);
let zoomOutButton: HTMLButtonElement | null = $state(null);
let resetZoomButton: HTMLButtonElement | null = $state(null);

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
        const cachedPath = await invoke<string>('get_photo', {
          photoPath: currentPhotoUrl,
        });
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

// Calculate minimum zoom to fit image in viewport
const minZoom = $derived.by(() => {
  if (!imgElement || !containerElement) return FALLBACK_MIN_ZOOM;

  const containerRect = containerElement.getBoundingClientRect();
  const { naturalWidth, naturalHeight } = imgElement;

  if (
    !naturalWidth ||
    !naturalHeight ||
    !containerRect.width ||
    !containerRect.height
  ) {
    return FALLBACK_MIN_ZOOM;
  }

  // Calculate zoom to fit width and height
  const fitWidthZoom = containerRect.width / naturalWidth;
  const fitHeightZoom = containerRect.height / naturalHeight;

  // Use the smaller zoom to ensure image fits in both dimensions
  // Add a small margin (0.95) to ensure it fits comfortably
  return Math.min(fitWidthZoom, fitHeightZoom) * 0.95;
});

const isPannable = $derived.by(() => {
  // Explicitly depend on zoom to trigger reactivity
  if (!imgElement || !containerElement || !zoom) return false;

  const containerRect = containerElement.getBoundingClientRect();

  // Use natural dimensions scaled by zoom for accurate calculation
  // This avoids timing issues with getBoundingClientRect after zoom changes
  const scaledWidth = imgElement.naturalWidth * zoom;
  const scaledHeight = imgElement.naturalHeight * zoom;

  // Image is pannable if scaled dimensions are larger than container
  return (
    scaledWidth > containerRect.width || scaledHeight > containerRect.height
  );
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
    zoom = minZoom;
  }
});

// Reset zoom and pan when displayPhotoUrl changes
$effect(() => {
  if (displayPhotoUrl) {
    zoom = minZoom;
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
  const newZoom = Math.max(minZoom, Math.min(MAX_ZOOM, zoom * delta));

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

function zoomIn() {
  if (zoomInButton) {
    zoomInButton.classList.add('bg-surface-700-300');
    setTimeout(() => zoomInButton?.classList?.remove('bg-surface-700-300'), 80);
  }

  const oldZoom = zoom;
  const newZoom = Math.max(minZoom, Math.min(MAX_ZOOM, zoom * 1.2));

  if (newZoom === oldZoom) return;

  // Adjust pan to keep the viewport center fixed
  // Same logic as handleWheel but with mouseX=0, mouseY=0 (center of viewport)
  const zoomRatio = newZoom / oldZoom;
  panX = zoomRatio * panX;
  panY = zoomRatio * panY;

  zoom = newZoom;
}

function zoomOut() {
  if (zoomOutButton) {
    zoomOutButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => zoomOutButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }

  const oldZoom = zoom;
  const newZoom = Math.max(minZoom, Math.min(MAX_ZOOM, zoom / 1.2));

  if (newZoom === oldZoom) return;

  // Adjust pan to keep the viewport center fixed
  // Same logic as handleWheel but with mouseX=0, mouseY=0 (center of viewport)
  const zoomRatio = newZoom / oldZoom;
  panX = zoomRatio * panX;
  panY = zoomRatio * panY;

  zoom = newZoom;
}

function resetZoom() {
  if (resetZoomButton) {
    resetZoomButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => resetZoomButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }

  zoom = minZoom;
  panX = 0;
  panY = 0;
}

function panUp() {
  if (!isPannable || !imgElement || !containerElement) return;

  if (panUpButton) {
    panUpButton.classList.add('bg-surface-700-300');
    setTimeout(() => panUpButton?.classList?.remove('bg-surface-700-300'), 80);
  }
  const imgRect = imgElement.getBoundingClientRect();
  const containerRect = containerElement.getBoundingClientRect();
  const maxPanY = Math.max(0, (imgRect.height - containerRect.height) / 2);

  panY = Math.max(-maxPanY, Math.min(maxPanY, panY + PAN_INCREMENT));
}

function panDown() {
  if (!isPannable || !imgElement || !containerElement) return;

  if (panDownButton) {
    panDownButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => panDownButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }
  const imgRect = imgElement.getBoundingClientRect();
  const containerRect = containerElement.getBoundingClientRect();
  const maxPanY = Math.max(0, (imgRect.height - containerRect.height) / 2);

  panY = Math.max(-maxPanY, Math.min(maxPanY, panY - PAN_INCREMENT));
}

function panLeft() {
  if (!isPannable || !imgElement || !containerElement) return;

  if (panLeftButton) {
    panLeftButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => panLeftButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }
  const imgRect = imgElement.getBoundingClientRect();
  const containerRect = containerElement.getBoundingClientRect();
  const maxPanX = Math.max(0, (imgRect.width - containerRect.width) / 2);

  panX = Math.max(-maxPanX, Math.min(maxPanX, panX + PAN_INCREMENT));
}

function panRight() {
  if (!isPannable || !imgElement || !containerElement) return;

  if (panRightButton) {
    panRightButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => panRightButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }
  const imgRect = imgElement.getBoundingClientRect();
  const containerRect = containerElement.getBoundingClientRect();
  const maxPanX = Math.max(0, (imgRect.width - containerRect.width) / 2);

  panX = Math.max(-maxPanX, Math.min(maxPanX, panX - PAN_INCREMENT));
}

function navigateNext() {
  if (photos.length === 0) return;

  if (navNextButton) {
    navNextButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => navNextButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }
  currentIndex = (currentIndex + 1) % photos.length;
}

function navigatePrevious() {
  if (photos.length === 0) return;

  if (navPrevButton) {
    navPrevButton.classList.add('bg-surface-700-300');
    setTimeout(
      () => navPrevButton?.classList?.remove('bg-surface-700-300'),
      80,
    );
  }
  currentIndex = (currentIndex - 1 + photos.length) % photos.length;
}

function handleMouseDown(e: MouseEvent) {
  if (!isPannable) return;

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
  if (
    !didDrag &&
    !isPannable &&
    zoom < MAX_ZOOM &&
    imgElement &&
    containerElement
  ) {
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
    zoom = MAX_ZOOM;

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
    <Dialog.Positioner class="fixed inset-0 z-[60] flex items-center justify-center">
      <Dialog.Content
        class="relative w-full h-full flex items-center justify-center"
        tabindex={-1}
        onkeydown={(e) => {
          // Handle all keyboard shortcuts at dialog level via event bubbling
          // These work regardless of which child element has focus

          // Arrow keys: context-aware (pan vs navigate)
          if (e.key === 'ArrowLeft') {
            e.preventDefault();
            if (isPannable) {
              panLeft();
            } else {
              navigatePrevious();
            }
          } else if (e.key === 'ArrowRight') {
            e.preventDefault();
            if (isPannable) {
              panRight();
            } else {
              navigateNext();
            }
          } else if (e.key === 'ArrowUp' && isPannable) {
            e.preventDefault();
            panUp();
          } else if (e.key === 'ArrowDown' && isPannable) {
            e.preventDefault();
            panDown();
          }
          // Zoom controls
          else if (e.key === '+' || e.key === '=') {
            e.preventDefault();
            zoomIn();
          } else if (e.key === '-') {
            e.preventDefault();
            zoomOut();
          } else if (e.key === '0') {
            e.preventDefault();
            resetZoom();
          }
        }}
      >
        <Dialog.CloseTrigger class={`${BTN_BASE_CLASSES.join(' ')} top-4 right-4`}>
          <X size={24} />
          Close
        </Dialog.CloseTrigger>

        {#if photos.length > 1 && !isPannable}
          <button
            bind:this={navPrevButton}
            type="button"
            class={`${NAV_BTN_CLASSES.join(' ')} left-4`}
            onclick={navigatePrevious}
            aria-label="Previous photo"
            title="Previous photo (&larr;)"
          >
            <ChevronLeft size={24} />
          </button>

          <button
            bind:this={navNextButton}
            type="button"
            class={`${NAV_BTN_CLASSES.join(' ')} right-4`}
            onclick={navigateNext}
            aria-label="Next photo"
            title="Next photo (&rarr;)"
          >
            <ChevronRight size={24} />
          </button>
        {/if}

        {#if displayPhotoUrl}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            bind:this={containerElement}
            class="overflow-auto w-full h-full flex items-center justify-center"
            role="application"
            tabindex="0"
            onwheel={handleWheel}
            onmousedown={handleMouseDown}
            onmousemove={handleMouseMove}
            onmouseup={handleMouseUp}
            onmouseleave={handleMouseUp}
            aria-label="Photo zoom and pan container"
          >
            <img
              bind:this={imgElement}
              src={displayPhotoUrl}
              alt="Full size"
              class="max-w-none"
              style="transform: translate({panX}px, {panY}px) scale({zoom}); cursor: {isDragging ? 'grabbing' : isPannable ? 'grab' : zoom > 1 ? 'zoom-out' : 'zoom-in'};"
            />
          </div>

          <!-- Zoom Controls -->
          <button
            bind:this={zoomInButton}
            type="button"
            class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-34 right-4`}
            onclick={zoomIn}
            disabled={zoom >= MAX_ZOOM}
            aria-label="Zoom in"
            title="Zoom in (+)"
          >
            <ZoomIn size={20} />
          </button>
          <button
            bind:this={zoomOutButton}
            type="button"
            class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-20 right-4`}
            onclick={zoomOut}
            disabled={zoom <= minZoom}
            aria-label="Zoom out"
            title="Zoom out (-)"
          >
            <ZoomOut size={20} />
          </button>
          <button
            bind:this={resetZoomButton}
            type="button"
            class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-6 right-4`}
            onclick={resetZoom}
            disabled={zoom === minZoom && panX === 0 && panY === 0}
            aria-label="Reset zoom"
            title="Reset zoom (0)"
          >
            <Maximize size={20} />
          </button>

          <!-- Pan Controls (only when pannable) -->
          {#if isPannable}
            <button
              bind:this={panUpButton}
              type="button"
              class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-20 left-18`}
              onclick={panUp}
              aria-label="Pan up"
              title="Pan up (&uarr;)"
            >
              <ArrowUp size={20} />
            </button>
            <button
              bind:this={panLeftButton}
              type="button"
              class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-6 left-4`}
              onclick={panLeft}
              aria-label="Pan left"
              title="Pan left (&larr;)"
            >
              <ArrowLeftIcon size={20} />
            </button>
            <button
              bind:this={panRightButton}
              type="button"
              class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-6 left-32`}
              onclick={panRight}
              aria-label="Pan right"
              title="Pan right (&rarr;)"
            >
              <ArrowRightIcon size={20} />
            </button>
            <button
              bind:this={panDownButton}
              type="button"
              class={`${CONTROL_BTN_CLASSES.join(' ')} bottom-6 left-18`}
              onclick={panDown}
              aria-label="Pan down"
              title="Pan down (&darr;)"
            >
              <ArrowDown size={20} />
            </button>
          {/if}

          <div
            class="
              absolute
              bottom-4
              left-1/2
              -translate-x-1/2
              bg-black/50
              text-white
              px-4
              py-2
              rounded
            "
          >
            Zoom: {Math.round(zoom * 100)}%
            {#if photos.length > 1}
              • {currentIndex + 1} / {photos.length} photos
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
