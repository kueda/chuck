<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMount } from 'svelte';
import OccurrenceCard, {
  EST_HEIGHT as CARD_HEIGHT,
} from '$lib/components/OccurrenceCard.svelte';
import type { Occurrence, SearchResult } from '$lib/types/archive';
import type { SearchParams } from '$lib/utils/filterCategories';

interface Props {
  groupValue: string | null;
  groupCount: number;
  searchParams: SearchParams;
  fieldName?: string;
  onClick: (occurrence: Occurrence) => void;
  onCountClick: () => void;
}

const {
  groupValue,
  groupCount,
  searchParams,
  fieldName,
  onClick,
  onCountClick,
}: Props = $props();

let occurrences = $state<Occurrence[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let containerElement: HTMLDivElement;

async function loadOccurrences() {
  if (!fieldName || loading || occurrences.length > 0) return;

  loading = true;
  error = null;

  try {
    // Build search params with filter for this group's value
    const filterValue = groupValue ?? '';
    const params: SearchParams = {
      ...searchParams,
      [fieldName]: filterValue,
    };

    const result = await invoke<SearchResult>('search', {
      limit: 5,
      offset: 0,
      searchParams: params,
      fields: undefined,
    });

    occurrences = result.results;
  } catch (err) {
    error = err instanceof Error ? err.message : String(err);
    occurrences = [];
  } finally {
    loading = false;
  }
}

onMount(() => {
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          observer.disconnect();
          loadOccurrences();
          break;
        }
      }
    },
    {
      rootMargin: '100%', // Start loading before element enters viewport
    },
  );

  observer.observe(containerElement);

  return () => {
    observer.disconnect();
  };
});
</script>

<div bind:this={containerElement} class="space-y-2">
  <div class="flex flex-row items-baseline justify-between">
    <h3 class="text-lg">{groupValue || '[None]'}</h3>
    <button
      type="button"
      class="btn btn-sm hover:preset-tonal-primary"
      onclick={onCountClick}
    >
      {groupCount.toLocaleString()} occurrences
    </button>
  </div>

  <div style={`height: ${CARD_HEIGHT}px`}>
    {#if loading}
      <p class="text-surface-500 text-sm w-full h-full flex  justify-center items-center">Loading occurrences...</p>
    {:else if error}
      <p class="text-error-500 text-sm w-full h-full flex  justify-center items-center">
        Error: {error}
      </p>
    {:else if occurrences.length === 0}
      <p class="text-surface-500 text-sm w-full h-full flex  justify-center items-center">
        No occurrences to display
      </p>
    {:else}
      <div class="grid gap-4 grid-cols-5">
        {#each occurrences as occurrence}
          <button
            type="button"
            class="text-start"
            onclick={() => onClick(occurrence)}
          >
            <OccurrenceCard {occurrence} />
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>
