<script lang="ts">
import type { MetaField } from '$lib/utils/xmlParser';

interface Props {
  fields: MetaField[];
}

const { fields }: Props = $props();
</script>

<table class="table">
  <thead>
    <tr>
      <th>Index</th>
      <th>Term</th>
      <th>Default</th>
      <th>Vocabulary</th>
    </tr>
  </thead>
  <tbody>
    {#each fields as field}
      <tr>
        <td>{field.index}</td>
        <td>
          {#if field.term.match(/^http/)}
            <a href={field.term} target="_blank"><code>{field.term}</code></a>
          {:else}
            <code>{field.term}</code>
          {/if}
        </td>
        <td>{field.default}</td>
        <td>{field.vocabulary}</td>
      </tr>
    {/each}
  </tbody>
</table>