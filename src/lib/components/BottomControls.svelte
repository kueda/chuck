<script lang="ts">
// Maybe excessively simple wrapper for the bottom controls
import type { Snippet } from 'svelte';

interface Props {
  children: Snippet;
}

const { children }: Props = $props();
</script>

<div class="bg-white shrink justify-between flex flex-row items-center gap-2 pt-2">
  {@render children()}
</div>
