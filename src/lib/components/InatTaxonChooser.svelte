<script lang="ts">
import { Leaf } from 'lucide-svelte';
import type { ComboboxItem, SearchResult, Taxon } from '$lib/types/inaturalist';
import InatSearchChooser from './InatSearchChooser.svelte';
import InatTaxonName from './InatTaxonName.svelte';

interface Props {
  selectedId?: number | null;
  onChange?: () => void;
}

let { selectedId = $bindable(), onChange }: Props = $props();

const mapTaxonResult = (result: SearchResult): ComboboxItem => {
  const taxon = (result.taxon || result.record) as Taxon;
  return {
    label: `${taxon.preferred_common_name || taxon.name} (${taxon.preferred_common_name ? taxon.name : taxon.iconic_taxon_name})`,
    value: taxon.id.toString(),
    item: taxon,
  };
};
</script>

<InatSearchChooser
  bind:selectedId
  {onChange}
  source="taxa"
  mapResultFn={mapTaxonResult}
  placeholder="Taxon"
  label="Taxon"
>
  {#snippet thumbnail({ selectedItem })}
    {@const taxon = selectedItem as Taxon | null}
    {#if taxon}
      {#if taxon.default_photo}
        <img
          src={taxon.default_photo.medium_url}
          alt={taxon.name}
          class="aspect-square h-9 object-cover rounded"
        />
      {:else}
        <div
          class="aspect-square h-9 bg-surface-500 rounded flex items-center justify-center text-surface-contrast-500"
        >
          <Leaf size={16} />
        </div>
      {/if}
    {:else}
      <div
        class="aspect-square h-9 bg-surface-200-800 rounded flex items-center justify-center"
      >
        <Leaf size={16} />
      </div>
    {/if}
  {/snippet}

  {#snippet itemContent({ item })}
    {@const taxon = item.item as Taxon}
    <div class="flex w-full gap-2">
      {#if taxon?.default_photo}
        <img
          src={taxon.default_photo.medium_url}
          alt={taxon.name}
          class="w-12 h-12 object-cover rounded"
        />
      {:else}
        <div class="w-12 h-12 bg-surface-200-800 rounded"></div>
      {/if}
      <div class="flex-1 items-start">
        <InatTaxonName {taxon} numLines={2} />
      </div>
    </div>
  {/snippet}
</InatSearchChooser>
