<script lang="ts">
  import { ExternalLink } from 'lucide-svelte';

  interface Props {
    value: boolean;
    name: string;
    title: string;
    desc: string;
    url: string;
  }

  let {
    value = $bindable(false),
    name,
    title,
    desc,
    url,
  }: Props = $props();
</script>

<label class="flex items-start w-fit space-x-2">
  <input name={name} class="checkbox mt-1" type="checkbox" bind:checked={value} />
  <div>
    <p class="flex flex-wrap space-x-2">
      <span>{title}</span>
      <a
        class="anchor flex items-center space-x-1"
        href={url}
        target="_blank"
      >
        <span>Docs</span>
        <ExternalLink size={16} />
      </a>
    </p>
    <p class="text-gray-500">{desc}</p>
  </div>
</label>
