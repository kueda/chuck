<script lang="ts">
import {
  Combobox,
  Portal,
  useListCollection,
} from '@skeletonlabs/skeleton-svelte';
import { XIcon } from 'lucide-svelte';
import type { Snippet } from 'svelte';
import { tick } from 'svelte';
import { useInatSearch } from '$lib/composables/useInatSearch.svelte.js';
import type {
  ComboboxItem,
  InatItem,
  SearchResult,
  SourceType,
} from '$lib/types/inaturalist';

interface Props {
  selectedId?: number | string | null;
  onChange?: () => void;
  source: SourceType;
  mapResultFn: (result: SearchResult) => ComboboxItem;
  placeholder?: string;
  label?: string;
  thumbnail: Snippet<[{ selectedItem: InatItem | null }]>;
  itemContent: Snippet<[{ item: ComboboxItem }]>;
}

let {
  selectedId = $bindable(),
  onChange,
  source,
  mapResultFn,
  placeholder = 'Search...',
  label,
  thumbnail,
  itemContent,
}: Props = $props();

const search = useInatSearch(source, mapResultFn);

let inputValue = $state('');
let isOpen = $state(false);

// Create collection for combobox
const collection = $derived(
  useListCollection({
    items: search.comboboxData,
    itemToString: (item) => item.label,
    itemToValue: (item) => item.value,
  }),
);

// If we receive an ID but we don't yet have the actual object corresponding
// to that ID, we need to fetch it from the API
$effect(() => {
  if (selectedId && !search.selectedItem) {
    search.loadById(selectedId);
  }
});

// Sync input value when selected item changes
$effect(() => {
  if (search.selectedItem) {
    const item = search.comboboxData.find(
      (i) => i.value === search.selectedValue[0],
    );
    if (item) {
      inputValue = item.label;
    }
  }
});

function handleInputValueChange(e: { inputValue: string }) {
  inputValue = e.inputValue;
  search.handleInputValueChange(e);
}

function handleValueChange(e: { value: string[] }) {
  search.handleValueChange(e, (selected) => {
    selectedId = selected?.id ?? null;
    if (onChange) {
      onChange();
    }
  });

  if (e.value.length > 0) {
    isOpen = false;
    tick().then(() => {
      // Clear suggestions after selection
    });
  }
}

function handleClear(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  search.clearSelection();
  selectedId = null;
  inputValue = '';
  if (onChange) {
    onChange();
  }
}

function handleOpenChange(e: { open: boolean }) {
  isOpen = e.open;
}

// Control popover visibility
$effect(() => {
  if (
    inputValue.length < 2 ||
    (search.comboboxData.length === 0 && !search.loading) ||
    (search.comboboxData.length === 1 &&
      search.comboboxData[0].label === inputValue)
  ) {
    isOpen = false;
  } else if (search.comboboxData.length > 0 && !search.selectedItem) {
    isOpen = true;
  }
});
</script>

<div class="w-full">
  {#if label}
    <label for={`search-chooser-${source}`} class="block text-sm font-medium mb-1">
      {label}
    </label>
  {/if}
  <div class="flex flex-row gap-2 items-center">
    {@render thumbnail({ selectedItem: search.selectedItem })}
    <div class="relative flex-1">
      <Combobox
        {collection}
        value={search.selectedValue}
        {inputValue}
        open={isOpen}
        onInputValueChange={handleInputValueChange}
        onValueChange={handleValueChange}
        onOpenChange={handleOpenChange}
        positioning={{
          // If the popover flips above the input, you might not see the top
          // result
          flip: false
        }}
      >
        <Combobox.Control>
          <Combobox.Input
            class="input w-full pr-8"
            autocapitalize="off"
            autocorrect="off"
            {placeholder}
            id={`search-chooser-${source}`}
          />
          {#if search.selectedItem}
            <button
              type="button"
              class="absolute right-2 top-1/2 -translate-y-1/2 hover:bg-gray-100 dark:hover:bg-gray-700 p-1 rounded"
              onclick={handleClear}
              aria-label="Clear selection"
            >
              <XIcon size={14} />
            </button>
          {/if}
        </Combobox.Control>
        <Portal>
          <Combobox.Positioner>
            <Combobox.Content>
              {#if search.loading}
                <div class="text-sm text-gray-500">Loading...</div>
              {:else if search.comboboxData.length === 0 && inputValue.length >= 2}
                <div class="text-sm text-gray-500">No matches found</div>
              {:else}
                {#each search.comboboxData as item (item.value)}
                  <Combobox.Item {item}>
                    {@render itemContent({ item })}
                  </Combobox.Item>
                {/each}
              {/if}
            </Combobox.Content>
          </Combobox.Positioner>
        </Portal>
      </Combobox>
    </div>
  </div>
</div>
