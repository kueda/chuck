<script lang="ts">
  import type { MetaData } from '$lib/utils/xmlParser';
  import MetaFields from './MetaFields.svelte';

  interface Props {
    data: MetaData;
  }

  let { data }: Props = $props();
</script>

<div class="space-y-6">
  <section>
    <h2 class="text-lg font-semibold mb-2">Core File</h2>
    <div class="space-y-1">
      <p class="text-surface-700-300">
        <span class="font-medium">Type:</span> {data.core.rowType}
      </p>
      <p class="text-surface-700-300">
        <span class="font-medium">Location:</span> {data.core.location}
      </p>
      {#if data.core.idIndex !== undefined}
        <p class="text-surface-700-300">
          <span class="font-medium">ID Column Index:</span> {data.core.idIndex}
        </p>
      {/if}
    </div>

    {#if data.core.fields.length > 0}
      <div class="mt-3">
        <h3 class="font-medium mb-1">Fields ({data.core.fields.length})</h3>
        <MetaFields fields={data.core.fields} />
      </div>
    {/if}
  </section>

  {#if data.extensions.length > 0}
    <section>
      <h2 class="text-lg font-semibold mb-2">Extensions ({data.extensions.length})</h2>
      <div class="space-y-4">
        {#each data.extensions as extension}
          <div class="border-l-2 border-surface-300-700 pl-3">
            <h3 class="font-medium mb-1">{extension.rowType} Extension</h3>
            <div class="space-y-1 text-sm">
              <p class="text-surface-700-300">
                <span class="font-medium">Location:</span> {extension.location}
              </p>
              {#if extension.coreIdIndex !== undefined}
                <p class="text-surface-700-300">
                  <span class="font-medium">Core ID Index:</span> {extension.coreIdIndex}
                </p>
              {/if}
              {#if extension.fields.length > 0}
                <div class="mt-2">
                  <p class="font-medium">Fields ({extension.fields.length})</p>
                  <MetaFields fields={extension.fields} />
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}
</div>
