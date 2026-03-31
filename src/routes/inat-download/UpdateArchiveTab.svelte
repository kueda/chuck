<script lang="ts">
import { AlertCircle } from 'lucide-svelte';
import {
  type ChuckArchiveInfo,
  estimateMediaCount,
  getUpdateObservationCount,
  type MediaEstimate,
  readChuckArchiveInfo,
  showOpenDialog,
} from '$lib/tauri-api';
import {
  BYTES_PER_OBSERVATION,
  BYTES_PER_OBSERVATION_COMMENTS,
  BYTES_PER_OBSERVATION_IDENTIFICATIONS,
  BYTES_PER_OBSERVATION_MULTIMEDIA,
  BYTES_PER_PHOTO,
  BYTES_PER_SOUND,
  formatBytes,
} from './size-estimate';

interface Props {
  onupdatestart: (path: string) => void;
}

const { onupdatestart }: Props = $props();

let updateFilePath = $state<string | null>(null);
let updateArchiveInfo = $state<ChuckArchiveInfo | null>(null);
let updateArchiveError = $state<string | null>(null);
let updateObsCount = $state<number | null>(null);
let updateObsCountLoading = $state<boolean>(false);
let updateObsCountError = $state<string | null>(null);
let updateMediaEstimate = $state<MediaEstimate | null>(null);
let updateMediaEstimateLoading = $state<boolean>(false);

function calculateEstimatedUpdateSize(): number | null {
  if (updateObsCount === null || !updateArchiveInfo) return null;

  let sizeBytes = updateObsCount * BYTES_PER_OBSERVATION;
  if (updateArchiveInfo.extensions.includes('SimpleMultimedia')) {
    sizeBytes += updateObsCount * BYTES_PER_OBSERVATION_MULTIMEDIA;
  }
  if (updateArchiveInfo.extensions.includes('Identifications')) {
    sizeBytes += updateObsCount * BYTES_PER_OBSERVATION_IDENTIFICATIONS;
  }
  if (updateArchiveInfo.extensions.includes('Comments')) {
    sizeBytes += updateObsCount * BYTES_PER_OBSERVATION_COMMENTS;
  }
  if (
    updateArchiveInfo.has_media &&
    updateMediaEstimate &&
    updateMediaEstimate.sample_size > 0
  ) {
    const photosPerObs =
      updateMediaEstimate.photo_count / updateMediaEstimate.sample_size;
    const soundsPerObs =
      updateMediaEstimate.sound_count / updateMediaEstimate.sample_size;
    sizeBytes += Math.round(photosPerObs * updateObsCount) * BYTES_PER_PHOTO;
    sizeBytes += Math.round(soundsPerObs * updateObsCount) * BYTES_PER_SOUND;
  }
  return sizeBytes;
}

async function handlePickUpdateFile() {
  const result = await showOpenDialog({
    filters: [{ name: 'Darwin Core Archive', extensions: ['zip'] }],
    multiple: false,
  });
  if (!result) return;
  const path = Array.isArray(result) ? result[0] : result;
  updateFilePath = path;
  updateArchiveInfo = null;
  updateArchiveError = null;
  updateObsCount = null;
  updateObsCountError = null;
  updateMediaEstimate = null;
  try {
    updateArchiveInfo = await readChuckArchiveInfo(path);
  } catch (e) {
    updateArchiveError = e instanceof Error ? e.message : String(e);
    return;
  }

  if (updateArchiveInfo?.inat_query) {
    updateObsCountLoading = true;
    try {
      updateObsCount = await getUpdateObservationCount(path);
    } catch (e) {
      updateObsCountError = e instanceof Error ? e.message : String(e);
    } finally {
      updateObsCountLoading = false;
    }

    if (updateArchiveInfo.has_media) {
      updateMediaEstimateLoading = true;
      try {
        updateMediaEstimate = await estimateMediaCount(
          updateArchiveInfo.inat_query,
        );
      } catch {
        // best-effort; size estimate will omit media if unavailable
      } finally {
        updateMediaEstimateLoading = false;
      }
    }
  }
}
</script>

<div class="mb-6 space-y-4">
  <div>
    <p class="text-sm mb-3">
      Choose an existing Chuck archive to update with recently changed observations.
    </p>
    <button
      type="button"
      class={['btn text-sm', updateArchiveInfo?.inat_query ? 'preset-tonal' : 'preset-filled']}
      onclick={handlePickUpdateFile}
    >
      {updateArchiveInfo?.inat_query ? 'Choose different file' : 'Choose archive…'}
    </button>
  </div>

  {#if updateArchiveError}
    <div class="p-3 border border-red-300 rounded text-red-600 text-sm">
      {updateArchiveError}
    </div>
  {:else if updateArchiveInfo}
    <div class="p-3 border rounded text-sm">
      <table class="table">
        <tbody>
          <tr>
            <td class="font-semibold">Location</td>
            <td>
              <span class="break-all">{updateFilePath}</span>
            </td>
          </tr>
          <tr>
            <td class="font-semibold">Filters</td>
            <td>
              {#if updateArchiveInfo.inat_query}
                <code class="break-all">{updateArchiveInfo.inat_query}</code>
              {:else}
                <span class="badge preset-filled-error-400-600">
                  <AlertCircle size={16} />
                  <span>Archive does not specify filters</span>
                </span>
              {/if}
            </td>
          </tr>
          <tr>
            <td class="font-semibold">Extensions</td>
            <td>
              {#if updateArchiveInfo.extensions.length > 0}
                <span>{updateArchiveInfo.extensions.join(', ')}</span>
              {:else}
                <span class="text-gray-500">none</span>
              {/if}
            </td>
          </tr>
          <tr>
            <td class="font-semibold">Media included</td>
            <td>{updateArchiveInfo.has_media ? 'Yes' : 'No'}</td>
          </tr>
          <tr>
            <td class="font-semibold">Current size</td>
            <td>{formatBytes(updateArchiveInfo.file_size_bytes)}</td>
          </tr>
          <tr>
            <td class="font-semibold">Last updated</td>
            <td>
              {
                updateArchiveInfo.pub_date
                  ? new Intl.DateTimeFormat(undefined, { dateStyle: "full" }).format(Date.parse(updateArchiveInfo.pub_date))
                  : "Unknown"
              }
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    {#if updateArchiveInfo.inat_query}
      <div class="p-3 border rounded text-sm">
        {#if updateObsCountLoading}
          <div class="text-gray-600">Counting updated observations…</div>
        {:else if updateObsCountError}
          <div class="text-red-600">Unable to count updated observations</div>
        {:else if updateObsCount !== null}
          <div>{updateObsCount.toLocaleString()} observations to update</div>
          {#if updateObsCount > 0}
            {#if updateMediaEstimateLoading}
              <div class="text-gray-500 text-sm mt-1">Estimating additions…</div>
            {:else}
              {@const estimatedAdditions = calculateEstimatedUpdateSize()}
              {#if estimatedAdditions !== null}
                <div class="text-gray-600 text-sm mt-1">
                  Estimated additions: up to {formatBytes(estimatedAdditions)}
                </div>
              {/if}
            {/if}
          {/if}
        {/if}
      </div>
    {/if}
  {/if}
</div>

<button
  type="button"
  class="btn preset-filled w-full"
  disabled={!updateFilePath || !!updateArchiveError || !updateArchiveInfo?.inat_query}
  onclick={() => updateFilePath && onupdatestart(updateFilePath)}
>
  Update Archive
</button>
