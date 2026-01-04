<script lang="ts">
  import {
    User
  } from 'lucide-svelte';

  interface Props {
    id?: string | null;
    name?: string | null;
  }
  const { id, name }: Props = $props();
  let displayName = $derived.by(() => {
    if (!name) return 'unknown';
    const pieces = name.split('|');
    let final = pieces.shift();
    if (pieces.length > 0) {
      return `${final} (${pieces.join(', ')})`
    }
    return final;
  });
  let url = $derived.by(() => {
    if (!id) return null;
    return id.split('|').find(s => s.startsWith('http'));
  });
</script>

<span class="flex flex-row items-center gap-1 overflow-hidden" title={id}>
  <User size={16} />
  <span class="shrink-4 text-nowrap overflow-hidden overflow-ellipsis">
    {#if url}
      <a class="link" href={url} target="_blank">{displayName}</a>
    {:else}
      {displayName}
    {/if}
  </span>
</span>
