<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from "svelte";

  interface ArchiveInfo {
    name: string,
    coreCount: number,
  }

  let archive = $state<ArchiveInfo>();

  onMount(async () => {
    try {
      archive = await invoke('current_archive');
      console.log('[+page.svelte] archive', archive);
    } catch (e) {
      console.log('[+page.svelte] no archive, e', e);
    }
  });
</script>

<div class="flex flex-col p-4">
  <header class="flex items-center justify-between">
    <p>{archive?.name || "no archive open"}</p>
    <button
      type="button"
      class="btn preset-filled"
      onclick={async () => {
        const path = await open();
        archive = await invoke('open_archive', { path });
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