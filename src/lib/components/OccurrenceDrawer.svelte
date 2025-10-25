<script lang="ts">
  import { Dialog, Portal } from '@skeletonlabs/skeleton-svelte';
  import { invoke } from '$lib/tauri-api';
  import type { Occurrence } from '$lib/types/archive';
  import PhotoViewer from './PhotoViewer.svelte';
  import OccurrenceMap from './OccurrenceMap.svelte';
  import { ArrowLeft, ArrowLeftCircle, ArrowRight, ArrowRightCircle, Calendar, Globe, Heading, MapPin, User, X } from 'lucide-svelte';

  interface Props {
    open: boolean;
    occurrenceId: string | number | null;
    coreIdColumn: string;
    onClose: () => void;
    onPrevious?: () => void;
    onNext?: () => void;
  }

  let {
    open = $bindable(false),
    occurrenceId,
    coreIdColumn,
    onClose,
    onPrevious,
    onNext
  }: Props = $props();

  let occurrence = $state<Occurrence | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let photoViewerOpen = $state(false);
  let photoUrls = $state<string[]>([]);
  let selectedPhotoIndex = $state(0);

  // Load occurrence when occurrenceId changes
  $effect(() => {
    if (occurrenceId && open) {
      loadOccurrence();
    }
  });

  async function loadOccurrence() {
    if (!occurrenceId) return;

    loading = true;
    error = null;

    try {
      const result = await invoke<Occurrence>('get_occurrence', {
        occurrenceId: String(occurrenceId)
      });
      occurrence = result;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      console.error('Error loading occurrence:', e);
    } finally {
      loading = false;
    }
  }

  function openPhotoViewer(photoUrl: string) {
    // Build array of all photo URLs from multimedia and audiovisual
    const allPhotos: string[] = [];

    // Add multimedia photos
    if (occurrence?.multimedia) {
      for (const media of occurrence.multimedia) {
        const url = getPhotoUrl(media);
        if (url) allPhotos.push(url);
      }
    }

    // Add audiovisual photos
    if (occurrence?.audiovisual) {
      for (const av of occurrence.audiovisual) {
        const url = getPhotoUrl(av);
        if (url) allPhotos.push(url);
      }
    }

    // Find index of clicked photo
    selectedPhotoIndex = allPhotos.indexOf(photoUrl);
    if (selectedPhotoIndex === -1) selectedPhotoIndex = 0;

    photoUrls = allPhotos;
    photoViewerOpen = true;
  }

  // Helper to get photo URL from multimedia or audiovisual
  function getPhotoUrl(item: any): string | null {
    return item.identifier || item.accessURI || null;
  }

  // Core fields to display in main view
  const coreFields = {
    what: [
      'scientificName',
      'vernacularName',
      'taxonRank',
      'taxonomicStatus',
      'kingdom',
      'phylum',
      'class',
      'order',
      'superfamily',
      'family',
      'subfamily',
      'tribe',
      'subtribe',
      'genus',
      'subgenus',
      'species',,
      'specificEpithet',
      'infraspecificEpithet',
      'cultivarEpithet'
    ],
    where: ['decimalLatitude', 'decimalLongitude', 'coordinateUncertaintyInMeters', 'locality', 'stateProvince', 'country'],
    when: ['eventDate', 'eventTime', 'year', 'month', 'day'],
    who: ['recordedBy', 'recordedById', 'identifiedBy', 'dateIdentified']
  };
</script>

<Dialog
  {open}
  closeOnInteractOutside={true}
  onOpenChange={(details) => { open = details.open; if (!details.open) onClose(); }}
>
  <Portal>
    <Dialog.Backdrop
      class="fixed inset-0 z-50 bg-black/50"
    />
    <Dialog.Positioner class="fixed inset-0 z-50 flex justify-end">
      <Dialog.Content
        class="h-screen w-[70%] bg-white dark:bg-gray-800 shadow-xl overflow-y-auto animate-slide-in-right"
        tabindex={0}
        onkeydown={(e) => {
          // Only handle arrow keys when PhotoViewer is not open
          if (photoViewerOpen) return;

          if (e.key === 'ArrowLeft' && onPrevious) {
            e.preventDefault();
            onPrevious();
          } else if (e.key === 'ArrowRight' && onNext) {
            e.preventDefault();
            onNext();
          }
        }}
      >
        <header class="sticky top-0 bg-white dark:bg-gray-800 px-6 py-4 flex items-center justify-between z-10">
          <div class="flex items-center gap-4">
            {#if onPrevious}
              <button
                type="button"
                class="btn btn-sm preset-outlined-surface-300-700 shadow"
                onclick={onPrevious}
              >
                <ArrowLeft size={20} />
                <span>Prev</span>
              </button>
            {/if}
            {#if onNext}
              <button
                type="button"
                class="btn btn-sm preset-outlined-surface-300-700 shadow flex-row-reverse"
                onclick={onNext}
              >
                <ArrowRight size={20} />
                Next
              </button>
            {/if}
          </div>
          <div>{coreIdColumn}: {occurrenceId}</div>
          <Dialog.CloseTrigger class="btn btn-sm">
            <X size={20} />
            Close
          </Dialog.CloseTrigger>
        </header>

        <!-- Content -->
        <div class="px-6 pb-6">
          {#if loading}
            <div class="flex justify-center items-center h-64">
              <div class="text-gray-500">Loading...</div>
            </div>
          {:else if error}
            <div class="text-red-600">Error: {error}</div>
          {:else if occurrence}
            <div class="mb-4">
              <h1 class="text-2xl font-bold mb-2">
                {#if occurrence.vernacularName && occurrence.vernacularName.length > 0}
                  {occurrence.vernacularName}
                  (
                {/if}{#if
                  occurrence.taxonRank === "genus"
                  || occurrence.taxonRank === "species"
                  || occurrence.taxonRank === "subspecies"
                  || occurrence.taxonRank === "variety"
                }
                  <i>{occurrence.scientificName}</i>
                {:else}
                  {occurrence.taxonRank}
                  {occurrence.scientificName}
                {/if}{#if occurrence.vernacularName && occurrence.vernacularName.length > 0}
                  )
                {/if}
              </h1>
              <div class="flex text-gray-500 gap-4">
                <span class="flex flex-row items-center gap-2">
                  <User size={16} />
                  {occurrence.recordedBy}
                </span>
                <span class="flex flex-row items-center gap-2">
                  <Calendar size={16} />
                  {occurrence.eventDate}
                </span>
                <span class="flex flex-row items-center gap-2">
                  <MapPin size={16} />
                  {occurrence.verbatimLocality}
                </span>
              </div>
            </div>

            <!-- Photos Section -->
            {#if occurrence.multimedia?.length || occurrence.audiovisual?.length}
              <section class="mb-8">
                <h2 class="text-xl font-bold mb-4">Media</h2>
                <div class="grid grid-cols-2 gap-4">
                  {#each occurrence.multimedia || [] as media}
                    {@const photoUrl = getPhotoUrl(media)}
                    {#if photoUrl}
                      <button
                        type="button"
                        class="aspect-square overflow-hidden rounded border hover:opacity-80 transition-opacity"
                        onclick={() => openPhotoViewer(photoUrl)}
                      >
                        <img
                          src={photoUrl}
                          alt={media.title || 'Photo'}
                          class="w-full h-full object-cover"
                        />
                      </button>
                    {/if}
                  {/each}
                  {#each occurrence.audiovisual || [] as av}
                    {@const photoUrl = getPhotoUrl(av)}
                    {#if photoUrl}
                      <button
                        type="button"
                        class="aspect-square overflow-hidden rounded border hover:opacity-80 transition-opacity"
                        onclick={() => openPhotoViewer(photoUrl)}
                      >
                        <img
                          src={photoUrl}
                          alt={av.title || 'Photo'}
                          class="w-full h-full object-cover"
                        />
                      </button>
                    {/if}
                  {/each}
                </div>
              </section>
            {/if}

            <!-- What Section -->
            <section class="mb-8">
              <h2 class="text-xl font-bold mb-4">What</h2>
              <dl class="grid grid-cols-3 gap-4">
                {#each coreFields.what as field}
                  {#if occurrence[field as keyof Occurrence]}
                    <div>
                      <dt class="text-sm text-gray-500">{field}</dt>
                      <dd class="font-medium">{occurrence[field as keyof Occurrence]}</dd>
                    </div>
                  {/if}
                {/each}
              </dl>
            </section>

            <!-- Where Section -->
            <section class="mb-8">
              <h2 class="text-xl font-bold mb-4">Where</h2>
              {#if occurrence.decimalLatitude && occurrence.decimalLongitude}
                <div class="mb-4">
                  <OccurrenceMap
                    latitude={Number(occurrence.decimalLatitude)}
                    longitude={Number(occurrence.decimalLongitude)}
                    uncertainty={occurrence.coordinateUncertaintyInMeters
                      ? Number(occurrence.coordinateUncertaintyInMeters)
                      : undefined}
                  />
                </div>
              {/if}
              <dl class="grid grid-cols-3 gap-4">
                {#each coreFields.where as field}
                  {#if occurrence[field as keyof Occurrence]}
                    <div>
                      <dt class="text-sm text-gray-500">{field}</dt>
                      <dd class="font-medium">{occurrence[field as keyof Occurrence]}</dd>
                    </div>
                  {/if}
                {/each}
              </dl>
            </section>

            <!-- When Section -->
            <section class="mb-8">
              <h2 class="text-xl font-bold mb-4">When</h2>
              <dl class="grid grid-cols-2 gap-4">
                {#each coreFields.when as field}
                  {#if occurrence[field as keyof Occurrence]}
                    <div>
                      <dt class="text-sm text-gray-500">{field}</dt>
                      <dd class="font-medium">{occurrence[field as keyof Occurrence]}</dd>
                    </div>
                  {/if}
                {/each}
              </dl>
            </section>

            <!-- Who Section -->
            <section class="mb-8">
              <h2 class="text-xl font-bold mb-4">Who</h2>
              <dl class="grid grid-cols-2 gap-4">
                {#each coreFields.who as field}
                  {#if occurrence[field as keyof Occurrence]}
                    <div>
                      <dt class="text-sm text-gray-500">{field}</dt>
                      <dd class="font-medium">{occurrence[field as keyof Occurrence]}</dd>
                    </div>
                  {/if}
                {/each}
              </dl>
            </section>

            <!-- All Fields (expandable) -->
            <details class="mt-8">
              <summary class="text-lg font-semibold cursor-pointer">All Fields</summary>
              <dl class="mt-4 grid grid-cols-2 gap-4">
                {#each Object.entries(occurrence) as [key, value]}
                  {#if value && !['multimedia', 'audiovisual', 'identification'].includes(key)}
                    <div>
                      <dt class="text-sm text-gray-500">{key}</dt>
                      <dd class="font-medium text-sm break-words">{value}</dd>
                    </div>
                  {/if}
                {/each}
              </dl>
            </details>
          {/if}
        </div>
      </Dialog.Content>
    </Dialog.Positioner>
  </Portal>
</Dialog>

<PhotoViewer
  bind:open={photoViewerOpen}
  photos={photoUrls}
  initialIndex={selectedPhotoIndex}
/>

<style>
  :global(.animate-slide-in-right) {
    animation: slide-in-right 0.3s ease-out;
  }

  @keyframes slide-in-right {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }
</style>
