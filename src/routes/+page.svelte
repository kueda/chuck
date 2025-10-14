<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from "svelte";

  interface ArchiveInfo {
    name: string,
    coreCount: number,
  }

  interface Occurrence {
    occurrenceID: string,
    scientificName: string,
    decimalLatitude: number,
    decimalLongitude: number,
    eventDate: Date,
    eventTime: Date,
  }

  let archive = $state<ArchiveInfo>();
  let occurrences = $state<Array<Occurrence>>([]);

  $effect(() => {
    if (archive) {
      invoke('search').then(results => {
        occurrences = results as [Occurrence];
      }).catch(e => {
        console.log('[+page.svelte] no occurrences, e', e);
      })
    }
  });

  $effect(() => {
    if (archive) {
      getCurrentWindow().setTitle(`${archive.name} – ${archive.coreCount} records`);
    }
  })

  async function openArchive() {
    const path = await open();
    if (path) {
      archive = await invoke('open_archive', { path });
    }
  }

  onMount(async () => {
    try {
      archive = await invoke('current_archive');
      console.log('[+page.svelte] archive', archive);
    } catch (e) {
      console.log('[+page.svelte] no archive, e', e);
    }

    // Listen for menu events
    const unlisten = await listen('menu-open', openArchive);
    return () => {
      unlisten();
    };
  });
</script>

{#if archive}
  <div class="flex flex-row p-4">
    <aside class="mr-4">side bar</aside>
    <main>
      <table class="table">
        <thead>
          <tr>
            <th>occurrenceID</th>
            <th>scientificName</th>
            <th>lat</th>
            <th>lng</th>
            <th>eventDate</th>
            <th>eventTime</th>
          </tr>
        </thead>
        <tbody>
          {#each occurrences as occ}
            <tr>
              <td>{occ.occurrenceID}</td>
              <td>{occ.scientificName}</td>
              <td>{occ.decimalLatitude}</td>
              <td>{occ.decimalLongitude}</td>
              <td>{occ.eventDate}</td>
              <td>{occ.eventTime}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </main>
  </div>
{:else}
  <div class="w-full h-screen flex flex-col justify-center items-center p-4 text-center">
    <p class="w-3/4 mb-5">Chuck is an application for viewing archives of biodiversity occurrences called DarwinCore Archives. Open an existing archive to get started</p>
    <button
      type="button"
      class="btn preset-filled"
      onclick={openArchive}
    >
      Open Archive
    </button>
  </div>
{/if}
