<script lang="ts">
import { XIcon } from 'lucide-svelte';

interface Props {
  columnName: string;
  minValue: string;
  maxValue: string;
  includeBlank: boolean;
  onMinChange: (value: string) => void;
  onMaxChange: (value: string) => void;
  onIncludeBlankChange: (value: boolean) => void;
}

const {
  columnName,
  minValue,
  maxValue,
  includeBlank,
  onMinChange,
  onMaxChange,
  onIncludeBlankChange,
}: Props = $props();

const hasRange = $derived(minValue !== '' || maxValue !== '');

function handleClearMin() {
  onMinChange('');
}

function handleClearMax() {
  onMaxChange('');
}
</script>

<div class="mb-3 p-2 {hasRange ? 'bg-surface-100' : ''}">
  <span class="label-text text-sm">{columnName}</span>
  <div class="flex gap-2 mt-1">
    <div class="relative flex-1">
      <input
        type="number"
        class="input pr-7 w-full"
        placeholder="Min"
        value={minValue}
        oninput={(e) => onMinChange(e.currentTarget.value)}
        aria-label="{columnName} minimum"
      />
      {#if minValue}
        <button
          type="button"
          class="absolute right-2 top-1/2 -translate-y-1/2
            hover:bg-gray-100 p-1 rounded"
          onclick={handleClearMin}
          aria-label="Clear minimum"
        >
          <XIcon size={14} />
        </button>
      {/if}
    </div>
    <div class="relative flex-1">
      <input
        type="number"
        class="input pr-7 w-full"
        placeholder="Max"
        value={maxValue}
        oninput={(e) => onMaxChange(e.currentTarget.value)}
        aria-label="{columnName} maximum"
      />
      {#if maxValue}
        <button
          type="button"
          class="absolute right-2 top-1/2 -translate-y-1/2
            hover:bg-gray-100 p-1 rounded"
          onclick={handleClearMax}
          aria-label="Clear maximum"
        >
          <XIcon size={14} />
        </button>
      {/if}
    </div>
  </div>
  <label class="flex items-center gap-2 mt-2 text-sm">
    <input
      type="checkbox"
      class="checkbox"
      checked={includeBlank}
      disabled={!hasRange}
      onchange={(e) =>
        onIncludeBlankChange(e.currentTarget.checked)}
    />
    Include blank values
  </label>
</div>
