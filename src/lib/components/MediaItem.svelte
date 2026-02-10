<script lang="ts">
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { ImageOff } from 'lucide-svelte';
import { onMount } from 'svelte';
import { fade } from 'svelte/transition';
import type { Audiovisual, Multimedia } from '$lib/types/archive';

const {
  multimediaItem,
  audiovisualItem,
  alt,
}: {
  multimediaItem?: Multimedia;
  audiovisualItem?: Audiovisual;
  alt?: string;
} = $props();

let imageLoaded = $state(false);
let mediumSrc = $state('');
let containerElement: HTMLDivElement;

const altText = $derived(alt || multimediaItem?.description || '');
const imageUrl = $derived(
  (multimediaItem?.identifier?.match(/\.(jpe?g|gif|png|webp)/i) && multimediaItem?.identifier)
  || (audiovisualItem?.accessURI?.match(/\.(jpe?g|gif|png|webp)/i) && audiovisualItem?.accessURI),
);

// Check if a path is a local file path (not a URL)
function isLocalPath(path: string): boolean {
  return !path.startsWith('http://') && !path.startsWith('https://');
}

// For some image providers, we may be able to use a more appropriate image,
// e.g. a smaller one
function getImageUrl(url: string): string {
  if (
    url.includes('static.inaturalist.org') ||
    url.includes('inaturalist-open-data.s3.amazonaws.com')
  ) {
    return url.replace(/\/(square|small|medium|large|original)/, '/medium');
  }
  return url;
}

onMount(() => {
  if (imageUrl) {
    // Lazy load the image, i.e. only when on screen
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            observer.disconnect();

            (async () => {
              // Check if this is a local file path or a URL
              if (isLocalPath(imageUrl)) {
                // Get the cached photo path from Tauri
                try {
                  const cachedPath = await invoke<string>('get_photo', {
                    photoPath: imageUrl,
                  });
                  mediumSrc = convertFileSrc(cachedPath);
                } catch (error) {
                  console.error('Failed to load local photo:', imageUrl, error);
                  return;
                }
              } else {
                // Remote URL - use as-is (with potential optimization)
                mediumSrc = getImageUrl(imageUrl);
              }

              // Preload the image
              const img = new Image();
              img.onload = () => {
                imageLoaded = true;
              };
              img.src = mediumSrc;
            })();
            break;
          }
        }
      },
      {
        rootMargin: '100%',
      },
    );

    observer.observe(containerElement);

    return () => {
      observer.disconnect();
    };
  }
});
</script>

<div bind:this={containerElement}>
  {#if imageUrl}
    {#if imageLoaded}
      <img
        alt={altText}
        src={mediumSrc}
        class="w-full h-full object-cover absolute inset-0"
        transition:fade={{ duration: 200 }}
      />
    {/if}
  {:else}
    <ImageOff size={46} aria-label="No photo" />
  {/if}
</div>
