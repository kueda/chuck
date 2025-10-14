<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from '@tauri-apps/api/window';
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
  <div class="flex flex-row">
    <aside>side bar</aside>
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
</div>
