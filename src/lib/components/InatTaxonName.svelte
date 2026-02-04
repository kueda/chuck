<script lang="ts">
import type { Taxon } from '$lib/types/inaturalist';

interface Props {
  taxon: Taxon | null;
  class?: string;
  firstClass?: string;
  secondClass?: string;
  numLines?: number;
  sciFirst?: boolean;
}

let {
  taxon,
  class: className = '',
  firstClass = '',
  secondClass = '',
  numLines = 0,
  sciFirst = false,
}: Props = $props();

let computedFirstClass = $derived.by(() => {
  let c = firstClass;
  if (numLines > 1 && taxon?.preferred_common_name) {
    c += ' line-clamp-1';
  }
  return c;
});

let computedSecondClass = $derived.by(() => {
  let c = secondClass;
  if (numLines > 1 && taxon?.preferred_common_name) {
    c += ' text-xs line-clamp-1';
  }
  return c;
});

let computedClassName = $derived.by(() => {
  let c = className;
  if (numLines > 0) {
    c += ` line-clamp-${numLines}`;
  }
  return c;
});
</script>

{#snippet scientificName(t: Taxon)}
  {#if t.rank_level && t.rank_level > 10}
    <span class="capitalize">{t.rank}</span>
  {/if}
  {#if t.rank_level && t.rank_level <= 20}
    <i>{t.name}</i>
  {:else}
    {t.name}
  {/if}
{/snippet}

<div class={computedClassName}>
  {#if taxon}
    {#if taxon.preferred_common_name && taxon.preferred_common_name.length > 0}
      <div class={computedFirstClass}>
        {#if sciFirst}
          {@render scientificName(taxon)}
        {:else}
          {taxon.preferred_common_name}
        {/if}
      </div>
      <div class={computedSecondClass}>
        {#if sciFirst}
          {taxon.preferred_common_name}
        {:else}
          {@render scientificName(taxon)}
        {/if}
      </div>
    {:else}
      <div class={computedFirstClass}>
        {@render scientificName(taxon)}
      </div>
    {/if}
  {:else if numLines > 1}
    <div class={computedFirstClass}>Unknown</div>
  {/if}
</div>
