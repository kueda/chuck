<script lang="ts">
  export interface SearchParams {
    scientific_name?: string;
  }

  interface Props {
    onSearchChange: (params: SearchParams) => void;
  }

  let { onSearchChange }: Props = $props();

  let scientificName = $state<string>('');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  function handleInput() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    debounceTimer = setTimeout(() => {
      const params: SearchParams = {};
      if (scientificName.trim()) {
        params.scientific_name = scientificName.trim();
      }
      onSearchChange(params);
    }, 300);
  }
</script>

<aside class="p-4 w-64">
  <h2 class="text-lg font-bold mb-4">Filters</h2>
  <div class="mb-4">
    <label for="scientificName" class="block text-sm font-medium mb-2">
      Scientific Name
    </label>
    <input
      id="scientificName"
      type="text"
      class="input w-full"
      bind:value={scientificName}
      oninput={handleInput}
      placeholder="Search..."
    />
  </div>
</aside>
