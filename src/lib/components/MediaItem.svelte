<script lang="ts">
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { AudioLines, ImageOff } from 'lucide-svelte';
import { onMount } from 'svelte';
import { fade } from 'svelte/transition';
import type { Audiovisual, Multimedia } from '$lib/types/archive';
import { isSoundMedia } from '$lib/utils/media';
export { isSoundMedia };

const {
  multimediaItem,
  audiovisualItem,
  alt,
  inatImageSize,
  noInteraction,
}: {
  multimediaItem?: Multimedia;
  audiovisualItem?: Audiovisual;
  alt?: string;
  inatImageSize?: 'square' | 'small' | 'medium' | 'large' | 'original';
  noInteraction?: boolean;
} = $props();

let imageLoaded = $state(false);
let imageSrc = $state('');
let soundSrc = $state('');
let containerElement: HTMLDivElement;

const altText = $derived(alt || multimediaItem?.description || '');

// Detect if this item is a sound
const soundUrl = $derived(
  multimediaItem && isSoundMedia(multimediaItem)
    ? multimediaItem.identifier || null
    : null,
);

const imageUrl = $derived(
  soundUrl
    ? null
    : (multimediaItem?.identifier?.match(/\.(jpe?g|gif|png|webp)/i) &&
        multimediaItem?.identifier) ||
        (audiovisualItem?.accessURI?.match(/\.(jpe?g|gif|png|webp)/i) &&
          audiovisualItem?.accessURI),
);

// Check if a path is a local file path (not a URL)
function isLocalPath(path: string): boolean {
  return !path.startsWith('http://') && !path.startsWith('https://');
}

// For some image providers, we may be able to use a more appropriate image,
// e.g. a smaller one
function getImageUrl(
  url: string,
  options?: { inatImageSize?: string },
): string {
  if (
    url.includes('static.inaturalist.org') ||
    url.includes('inaturalist-open-data.s3.amazonaws.com')
  ) {
    if (options?.inatImageSize === 'square')
      return url.replace(/\/(square|small|medium|large|original)/, '/square');
    if (options?.inatImageSize === 'small')
      return url.replace(/\/(square|small|medium|large|original)/, '/small');
    if (options?.inatImageSize === 'large')
      return url.replace(/\/(square|small|medium|large|original)/, '/large');
    if (options?.inatImageSize === 'original')
      return url.replace(/\/(square|small|medium|large|original)/, '/original');
    return url.replace(/\/(square|small|medium|large|original)/, '/medium');
  }
  return url;
}

onMount(() => {
  if (soundUrl) {
    // Load sound source (local or remote)
    (async () => {
      if (isLocalPath(soundUrl)) {
        try {
          const cachedPath = await invoke<string>('get_photo', {
            photoPath: soundUrl,
          });
          soundSrc = convertFileSrc(cachedPath);
        } catch (error) {
          console.error('Failed to load local sound:', soundUrl, error);
        }
      } else {
        soundSrc = soundUrl;
      }
    })();
  } else if (imageUrl) {
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
                  imageSrc = convertFileSrc(cachedPath);
                } catch (error) {
                  console.error('Failed to load local photo:', imageUrl, error);
                  return;
                }
              } else {
                // Remote URL - use as-is (with potential optimization)
                imageSrc = getImageUrl(imageUrl, { inatImageSize });
              }

              // Preload the image
              const img = new Image();
              img.onload = () => {
                imageLoaded = true;
              };
              img.src = imageSrc;
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

<div bind:this={containerElement} class="w-full h-full justify-center flex items-center">
  {#if soundUrl}
    {#if noInteraction}
      <div class="flex items-center justify-center w-full h-full">
        <AudioLines size={64} />
      </div>
    {:else}
      {#if soundSrc}
        <audio controls src={soundSrc} class="w-full">
          Your browser does not support the audio element.
        </audio>
      {:else}
        <div class="flex items-center justify-center w-full h-full text-gray-400">
          Loading audio...
        </div>
      {/if}
    {/if}
  {:else if imageUrl}
    {#if imageLoaded}
      <img
        alt={altText}
        src={imageSrc}
        class="w-full h-full object-cover absolute inset-0"
        transition:fade={{ duration: 200 }}
      />
    {/if}
  {:else}
    <div class="w-1/2 h-1/2 text-gray-500 flex flex-col justify-center items-center text-center">
      <ImageOff size={46} aria-label="No photo" />
      {#if multimediaItem?.identifier || audiovisualItem?.accessURI}
        <p>Cannot load media</p>
        <p>{multimediaItem?.identifier || audiovisualItem?.accessURI }</p>
      {/if}
    </div>
  {/if}
</div>
