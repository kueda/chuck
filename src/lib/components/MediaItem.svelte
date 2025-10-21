<script lang="ts">
  import { ImageOff } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { Multimedia, Audiovisual } from '$lib/types/archive';

  let {
    media,
    alt
  }: {
    media?: Multimedia | Audiovisual;
    alt?: string;
  } = $props();

  let imageLoaded = $state(false);
  let mediumSrc = $state('');
  let containerElement: HTMLDivElement;

  const altText = $derived(alt || media?.description || '');
  const imageUrl = $derived(media?.identifier);

  // For some image providers, we may be able to use a more appropriate image,
  // e.g. a smaller one
  function getImageUrl(url: string): string {
    if (
      url.includes('static.inaturalist.org')
      || url.includes('inaturalist-open-data.s3.amazonaws.com')
    ) {
      return url.replace(/\/(square|small|medium|large|original)/, '/medium');
    }
    return url;
  }

  onMount(() => {
    if (imageUrl) {
      // Lazy load the image, i.e. only when on screen
      const observer = new IntersectionObserver((entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const img = new Image();
            mediumSrc = getImageUrl(imageUrl);
            img.onload = () => {
              imageLoaded = true;
            };
            img.src = mediumSrc;
            observer.disconnect();
          }
        });
      }, {
        rootMargin: '100%'
      });

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
