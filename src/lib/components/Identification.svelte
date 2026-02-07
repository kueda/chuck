<script lang="ts">
import { Info } from 'lucide-svelte';
import ActivityCard from '$lib/components/ActivityCard.svelte';
import Markup from '$lib/components/Markup.svelte';
import Taxon from '$lib/components/Taxon.svelte';

const { identification: ident } = $props();
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

<ActivityCard
  agentName={ident.identifiedBy}
  agentId={ident.identifiedByID}
  date={ident.dateIdentified}
  dimmed={ident.identificationCurrent === false}
>
  {#snippet children()}
    <Taxon item={ident} />
    <!-- <code>{JSON.stringify(ident, null, "\t")}</code> -->
    {#if ident.identificationRemarks}
      <Markup text={ident.identificationRemarks} />
    {/if}
  {/snippet}
  {#snippet footer()}
    {#if ident.identificationVerificationStatus
      || (Object.keys(ident).includes('identificationCurrent')
        && !ident.identificationCurrent)
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
        {#if Object.keys(ident).includes('identificationCurrent')
          && !ident.identificationCurrent
        }
          <div class="badge preset-filled-surface-200-800">Withdrawn</div>
        {/if}
      </footer>
    {/if}
  {/snippet}
</ActivityCard>
