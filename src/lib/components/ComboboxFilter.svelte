<script lang="ts">
import {
  Combobox,
  Portal,
  useListCollection,
} from '@skeletonlabs/skeleton-svelte';
import { XIcon } from 'lucide-svelte';
import { tick } from 'svelte';
import type { HTMLInputAttributes } from 'svelte/elements';
import { invoke } from '$lib/tauri-api';

interface Props {
  columnName: string;
  value: string;
  onValueChange: (value: string) => void;
  onClear: () => void;
  type?: HTMLInputAttributes['type'];
}

const {
  columnName,
  value,
  onValueChange,
  onClear,
  type = 'text',
}: Props = $props();

let inputValue = $state(value);
let selectedValue = $state<string[]>(value ? [value] : []); // Controlled value for Combobox
let suggestions = $state<string[]>([]);
let loading = $state(false);
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let containerRef = $state<HTMLDivElement>();
let isOpen = $state(false);

// Create collection for combobox
const collection = $derived(
  useListCollection({
    items: suggestions.map((s) => ({ value: s, label: s })),
    itemToString: (item) => item.label,
    itemToValue: (item) => item.value,
  }),
);

// Debounced autocomplete fetch
async function fetchSuggestions(searchTerm: string) {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }

  debounceTimer = setTimeout(async () => {
    if (searchTerm.length < 2) {
      suggestions = [];
      return;
    }

    loading = true;
    try {
      const results = await invoke<string[]>('get_autocomplete_suggestions', {
        columnName,
        searchTerm,
        limit: 50,
      });
      suggestions = results;
    } catch (error) {
      console.error('Failed to fetch autocomplete suggestions:', error);
      suggestions = [];
    } finally {
      loading = false;
    }
  }, 300);
}

let lastUserInput = $state('');
let ignoreNextClear = $state(false);

function handleInputValueChange(event: { inputValue: string }) {
  // If we're ignoring a clear and getting an empty string, restore the last user input
  if (event.inputValue === '' && ignoreNextClear && lastUserInput) {
    ignoreNextClear = false;
    inputValue = lastUserInput;
    onValueChange(lastUserInput);
    return;
  }

  // Save the last non-empty user input
  if (event.inputValue) {
    lastUserInput = event.inputValue;
  }

  inputValue = event.inputValue;
  if (type !== 'number') fetchSuggestions(event.inputValue);
  onValueChange(event.inputValue);
}

function handleValueChange(event: { value: string[] }) {
  selectedValue = event.value; // Update controlled value
  if (event.value.length > 0) {
    const newValue = event.value[0];
    inputValue = newValue;
    onValueChange(newValue);
    isOpen = false;
    // User just chose a value, no need to show suggestions anymore
    tick().then(() => {
      suggestions = [];
    });
  }
}

// Sync external value changes (but preserve local input if it has content)
$effect(() => {
  // Always sync when value changes - this ensures clear button works
  if (value !== inputValue) {
    inputValue = value;
    selectedValue = value ? [value] : [];
    lastUserInput = value;
  }
});

// Control popover visibility: close when input is empty or no suggestions
$effect(() => {
  if (
    // Don't bother showing suggestions for a single letter
    inputValue.length < 2 ||
    // No reason to show empty suggestions
    (suggestions.length === 0 && !loading) ||
    // Hide suggestions if there's only one and it matches the current value exactly
    (suggestions.length === 1 && suggestions[0] === inputValue)
  ) {
    isOpen = false;
  } else if (suggestions.length > 1) {
    isOpen = true;
  }
});

function handleOpenChange(event: { open: boolean }) {
  isOpen = event.open;
}

function handleInteractOutside() {
  // When user clicks outside, set flag to ignore the next clear
  if (lastUserInput) {
    ignoreNextClear = true;
  }
}
</script>

<div
  bind:this={containerRef}
  class="mb-3 p-2 {selectedValue.length > 0 && 'bg-surface-100'}"
>
  <label for={`Combobox-${columnName}`} class="label mb-1">
    <span class="label-text text-sm">{columnName}</span>
  </label>
  <div class="relative">
    <Combobox
      {collection}
      placeholder="Search..."
      value={selectedValue}
      inputValue={inputValue}
      open={isOpen}
      onInputValueChange={handleInputValueChange}
      onValueChange={handleValueChange}
      onOpenChange={handleOpenChange}
      onInteractOutside={handleInteractOutside}
    >
      <Combobox.Control>
        <Combobox.Input
          class="input pr-8"
          autocapitalize="off"
          autocorrect="off"
          type={type}
          id={`Combobox-${columnName}`}
        />
        {#if inputValue}
          <button
            type="button"
            class="absolute right-2 top-1/2 -translate-y-1/2 hover:bg-gray-100 p-1 rounded"
            onclick={onClear}
            aria-label="Clear filter"
          >
            <XIcon size={14} />
          </button>
        {/if}
      </Combobox.Control>
      <Portal>
        <Combobox.Positioner>
          <Combobox.Content class="card bg-surface-100-900 shadow-lg max-h-60 overflow-y-auto w-full">
            {#if loading}
              <div class="text-sm text-gray-500">Loading...</div>
            {:else if suggestions.length === 0 && inputValue.length >= 2}
              <div class="text-sm text-gray-500">No matches found</div>
            {:else}
              {#each suggestions as suggestion (suggestion)}
                <Combobox.Item item={{ value: suggestion, label: suggestion }}>
                  <Combobox.ItemText class="text-sm hover:bg-gray-100 cursor-pointer rounded">
                    {suggestion}
                  </Combobox.ItemText>
                  <Combobox.ItemIndicator />
                </Combobox.Item>
              {/each}
            {/if}
          </Combobox.Content>
        </Combobox.Positioner>
      </Portal>
    </Combobox>
  </div>
</div>
