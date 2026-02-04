<script lang="ts">
import { MapPin } from 'lucide-svelte';
import type { ComboboxItem, Place, SearchResult } from '$lib/types/inaturalist';
import InatSearchChooser from './InatSearchChooser.svelte';

interface Props {
  selectedId?: number | null;
  onChange?: () => void;
}

let { selectedId = $bindable(), onChange }: Props = $props();

const mapPlaceResult = (result: SearchResult): ComboboxItem => {
  const place = (result.place || result.record) as Place;
  return {
    label: `${place.display_name} (${place.id})`,
    value: place.id.toString(),
    item: place,
  };
};
</script>

<InatSearchChooser
  bind:selectedId
  {onChange}
  source="places"
  mapResultFn={mapPlaceResult}
  placeholder="Place"
  label="Place"
>
  {#snippet thumbnail({ selectedItem })}
    {#if selectedItem}
      <div
        class="aspect-square h-9 bg-surface-500 rounded flex items-center justify-center text-surface-contrast-500"
      >
        <MapPin size={16} />
      </div>
    {:else}
      <div
        class="aspect-square h-9 bg-surface-200-800 rounded flex items-center justify-center"
      >
        <MapPin size={16} />
      </div>
    {/if}
  {/snippet}

  {#snippet itemContent({ item })}
    {@const place = item.item as Place}
    <div class="flex w-full gap-2 items-center">
      <div
        class="aspect-square h-9 bg-surface-200-800 rounded flex items-center justify-center"
      >
        <MapPin size={16} />
      </div>
      <div class="flex-1">
        <div class="line-clamp-1 text-ellipsis font-semibold">
          {place?.display_name || item.label}
        </div>
        {#if place?.place_type}
          <div class="line-clamp-1 text-ellipsis text-sm capitalize">
            {place.place_type}
          </div>
        {/if}
      </div>
    </div>
  {/snippet}
</InatSearchChooser>
