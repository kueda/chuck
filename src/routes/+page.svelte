<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from '@tauri-apps/plugin-dialog';

  let name = $state("");
  let greetMsg = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke("greet", { name });
  }
</script>

<div class="flex flex-col p-4">
  <header class="flex items-center justify-between">
    <p>head stuff</p>
    <button
      type="button"
      class="btn preset-filled"
      onclick={async () => {
        const path = await open();
        const rowCount = await invoke('open_archive', { path });
        console.log('[+page.svelte] path', path);
        console.log('[+page.svelte] rowCount', rowCount);
      }}
    >
      Open Archive
    </button>
  </header>
  <div class="grid grid-cols-2">
    <aside>side bar</aside>
    <main>main stuff</main>
  </div>
</div>