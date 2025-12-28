<script lang="ts">
  import Agent from '$lib/components/Agent.svelte';
  import Taxon from '$lib/components/Taxon.svelte';
  import Markup from '$lib/components/Markup.svelte';
  import { Info } from 'lucide-svelte';

  let { identification: ident } = $props();

  let date = $derived.by(() => {
    if (ident.dateIdentified.match(/\d+:\d+:\d+/)) {
      const d = new Date(ident.dateIdentified);
      if (d.getTime()) return d;
    }
    return null;
  });
</script>

{#snippet uriLabel(uri: string)}
  {#if uri.includes('://')}
    <span class="flex flex-row gap-1 items-center">
      {uri.split('/').pop()}
      <a href={uri} target="_blank"><Info size={12} /></a>
    </span>
  {:else}
    <span>{uri}</span>
  {/if}
{/snippet}

<div
  class={[
    "card",
    "preset-filled-surface-100-900",
    "border-surface-200-800",
    "rounded-md",
    "border-2",
    "divide-surface-200-800",
    "w-full",
    "divide-y",
    "flex",
    "flex-col",
    {"opacity-50": ident.identificationCurrent === false}
  ]}
>
  <header class="p-2 text-sm flex justify-between">
    <Agent name={ident.identifiedBy} id={ident.identifiedByID} />
    {#if ident.dateIdentified}
      <time
        datetime={date ? ident.dateIdentified : ""}
        title={ident.dateIdentified}
      >
        {date ? date.toLocaleString() : `"${ident.dateIdentified}"`}
      </time>
    {/if}
  </header>
  <article class="p-2 flex flex-col gap-2">
    <Taxon item={ident} />
    <!-- <code>{JSON.stringify(ident, null, "\t")}</code> -->
    {#if ident.identificationRemarks}
      <Markup text={ident.identificationRemarks} />
    {/if}
  </article>
  {#if ident.identificationVerificationStatus
    || (Object.keys(ident).includes('identificationCurrent') && !ident.identificationCurrent)
  }
    <footer class="p-2 flex justify-between text-xs">
      {#if ident.identificationVerificationStatus === "https://www.inaturalist.org/terminology/supporting"}
        <div class="badge preset-filled-success-50-950">
          {@render uriLabel(ident.identificationVerificationStatus)}
        </div>
      {:else if ident.identificationVerificationStatus === "https://www.inaturalist.org/terminology/improving"}
        <div class="badge preset-filled-success-500">
          {@render uriLabel(ident.identificationVerificationStatus)}
        </div>
      {:else if ident.identificationVerificationStatus === "https://www.inaturalist.org/terminology/maverick"}
        <div class="badge preset-filled-error-200-800">
          {@render uriLabel(ident.identificationVerificationStatus)}
        </div>
      {:else if ident.identificationVerificationStatus === "https://www.inaturalist.org/terminology/leading"}
        <div class="badge preset-filled-warning-200-800">
          {@render uriLabel(ident.identificationVerificationStatus)}
        </div>
      {:else if ident.identificationVerificationStatus}
        <div class="badge preset-filled-surface-200-800">
          {@render uriLabel(ident.identificationVerificationStatus)}
        </div>
      {/if}
      {#if Object.keys(ident).includes('identificationCurrent') && !ident.identificationCurrent}
        <div class="badge preset-filled-surface-200-800">Withdrawn</div>
      {/if}
    </footer>
  {/if}
</div>
