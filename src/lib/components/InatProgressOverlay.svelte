<script lang="ts">
import { Progress } from '@skeletonlabs/skeleton-svelte';

interface Props {
  stage: 'active' | 'building' | 'complete' | 'error';
  observationsCurrent?: number;
  observationsTotal?: number;
  mediaCurrent?: number;
  mediaTotal?: number;
  mediaIsEstimate?: boolean;
  message?: string;
  estimatedTimeRemaining?: string;
  mergeCurrent?: number;
  mergeTotal?: number;
  onCancel: () => void;
}

const {
  stage,
  observationsCurrent,
  observationsTotal,
  mediaCurrent,
  mediaTotal,
  mediaIsEstimate = true,
  message,
  estimatedTimeRemaining,
  mergeCurrent,
  mergeTotal,
  onCancel,
}: Props = $props();

const observationsProgress = $derived.by(() => {
  if (observationsTotal && observationsTotal > 0) {
    return ((observationsCurrent || 0) / observationsTotal) * 100;
  }
  return undefined;
});

const mediaProgress = $derived.by(() => {
  if (mediaTotal && mediaTotal > 0) {
    return Math.min(100, ((mediaCurrent || 0) / mediaTotal) * 100);
  }
  return undefined;
});

const showObservations = $derived(
  stage === 'active' && observationsTotal !== undefined,
);
const showMedia = $derived(stage === 'active' && mediaTotal !== undefined);
const fmtr = Intl.NumberFormat(navigator.languages);

function fmtNumber(val: number | undefined) {
  return val ? fmtr.format(val) : '?';
}
</script>

{#snippet progressWithMessage(
  title: string,
  current: number | undefined,
  total: number | undefined,
  progress: number | undefined,
  isEstimate?: boolean
)}
  <div class="mb-4">
    <div class="text-sm mb-2 flex justify-between">
      <div>{title}</div>
      <div>{fmtNumber(current)} / {isEstimate ? ' ~' : ''}{fmtNumber(total)}</div>
    </div>
    <Progress value={progress} class="w-full">
      <Progress.Track>
        <Progress.Range />
      </Progress.Track>
    </Progress>
  </div>
{/snippet}

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
    <h2 class="text-xl font-bold mb-4">Generating Darwin Core Archive</h2>

    {#if stage === 'active'}
      {#if showObservations}
        {@render progressWithMessage(
          "Fetching observations...",
          observationsCurrent,
          observationsTotal,
          observationsProgress
        )}
      {/if}

      {#if showMedia}
        {@render progressWithMessage(
          "Downloading media...",
          mediaCurrent,
          mediaTotal,
          mediaProgress,
          mediaIsEstimate,
        )}
      {/if}

      {#if estimatedTimeRemaining}
        <div class="text-sm text-gray-600 dark:text-gray-400 mt-2 text-center">
          {estimatedTimeRemaining}
        </div>
      {/if}
    {:else if stage === 'building' && message}
      <div class="mb-4">
        <div class="text-sm mb-2 flex justify-between">
          <div>{message}</div>
          {#if mergeTotal}
            <div>{fmtNumber(mergeCurrent)} / {fmtNumber(mergeTotal)}</div>
          {/if}
        </div>
        <Progress
          value={mergeTotal ? ((mergeCurrent ?? 0) / mergeTotal) * 100 : undefined}
          class="w-full"
        >
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
