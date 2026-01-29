<script lang="ts">
import { Progress } from '@skeletonlabs/skeleton-svelte';

interface Props {
  stage: 'active' | 'building' | 'complete' | 'error';
  observationsCurrent?: number;
  observationsTotal?: number;
  photosCurrent?: number;
  photosTotal?: number;
  message?: string;
  estimatedTimeRemaining?: string;
  onCancel: () => void;
}

const {
  stage,
  observationsCurrent,
  observationsTotal,
  photosCurrent,
  photosTotal,
  message,
  estimatedTimeRemaining,
  onCancel,
}: Props = $props();

const observationsProgress = $derived.by(() => {
  if (observationsTotal && observationsTotal > 0) {
    return ((observationsCurrent || 0) / observationsTotal) * 100;
  }
  return undefined;
});

const photosProgress = $derived.by(() => {
  if (photosTotal && photosTotal > 0) {
    return ((photosCurrent || 0) / photosTotal) * 100;
  }
  return undefined;
});

const showObservations = $derived(
  stage === 'active' && observationsTotal !== undefined,
);
const showPhotos = $derived(stage === 'active' && photosTotal !== undefined);
</script>

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
    <h2 class="text-xl font-bold mb-4">Generating Darwin Core Archive</h2>

    {#if stage === 'active'}
      {#if showObservations}
        <div class="mb-4">
          <div class="text-sm mb-2">
            Fetching observations... {observationsCurrent}/{observationsTotal}
          </div>
          <Progress value={observationsProgress} class="w-full">
            <Progress.Track>
              <Progress.Range />
            </Progress.Track>
          </Progress>
        </div>
      {/if}

      {#if showPhotos}
        <div class="mb-4">
          <div class="text-sm mb-2">
            Downloading photos... {photosCurrent}/{photosTotal}
          </div>
          <Progress value={photosProgress} class="w-full">
            <Progress.Track>
              <Progress.Range />
            </Progress.Track>
          </Progress>
        </div>
      {/if}

      {#if estimatedTimeRemaining}
        <div class="text-sm text-gray-600 dark:text-gray-400 mt-2 text-center">
          {estimatedTimeRemaining}
        </div>
      {/if}
    {:else if stage === 'building' && message}
      <div class="mb-4">
        <div class="text-sm mb-2">{message}</div>
        <Progress value={undefined} class="w-full">
          <Progress.Track>
            <Progress.Range />
          </Progress.Track>
        </Progress>
      </div>
    {:else if stage === 'complete'}
      <div class="mb-4 text-sm">Complete!</div>
    {:else if stage === 'error' && message}
      <div class="mb-4 text-sm text-error-600">Error: {message}</div>
    {/if}

    {#if stage !== 'complete' && stage !== 'error'}
      <button
        type="button"
        class="btn preset-tonal w-full"
        onclick={onCancel}
      >
        Cancel
      </button>
    {/if}
  </div>
</div>
