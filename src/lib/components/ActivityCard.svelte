<script lang="ts">
import type { Snippet } from 'svelte';
import Agent from '$lib/components/Agent.svelte';

interface Props {
  agentName?: string | null;
  agentId?: string | null;
  date?: string | null;
  dimmed?: boolean;
  children: Snippet;
  footer?: Snippet;
}

const {
  agentName,
  agentId,
  date,
  dimmed = false,
  children,
  footer,
}: Props = $props();

const parsedDate = $derived.by(() => {
  if (!date) return null;
  if (date.match(/\d+:\d+:\d+/)) {
    const d = new Date(date);
    if (d.getTime()) return d;
  }
  return null;
});
</script>

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
    {"opacity-50": dimmed}
  ]}
>
  <header class="p-2 text-sm flex justify-between">
    <Agent name={agentName} id={agentId} />
    {#if date}
      <time
        datetime={parsedDate ? date : ""}
        title={date}
      >
        {parsedDate ? parsedDate.toLocaleString() : `"${date}"`}
      </time>
    {/if}
  </header>
  <article class="p-2 flex flex-col gap-2">
    {@render children()}
  </article>
  {#if footer}
    {@render footer()}
  {/if}
</div>
