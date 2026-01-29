<script lang="ts">
import { Switch, Tabs } from '@skeletonlabs/skeleton-svelte';
import { onMount } from 'svelte';
import EMLDisplay from '$lib/components/EMLDisplay.svelte';
import MetaDisplay from '$lib/components/MetaDisplay.svelte';
import { getCurrentWindow, invoke } from '$lib/tauri-api';
import type { ArchiveInfo } from '$lib/types/archive';
import type { EMLData, MetaData } from '$lib/utils/xmlParser';
import { parseEML, parseMeta, prettify } from '$lib/utils/xmlParser';

interface XmlFile {
  filename: string;
  content: string;
}

interface ArchiveMetadata {
  xml_files: XmlFile[];
}

type XmlFileType = 'eml' | 'meta' | 'raw';

interface ProcessedXmlFile {
  filename: string;
  content: string;
  type: XmlFileType;
  emlData?: EMLData | null;
  metaData?: MetaData | null;
}

let archive = $state<ArchiveInfo>();
let metadata = $state<ArchiveMetadata | null>(null);
let loading = $state<boolean>(true);
let error = $state<string | null>(null);
let activeTab = $state<string>('');
let viewingSource = $state<boolean>(false);

/**
 * Detect the type of XML based on its root element
 */
function detectXmlType(content: string): XmlFileType {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(content, 'text/xml');

    if (doc.querySelector('parsererror')) {
      return 'raw';
    }

    const root = doc.documentElement;
    if (!root) {
      return 'raw';
    }

    // Check for EML root element (can be <eml:eml> or just <eml>)
    if (root.localName === 'eml' || root.tagName.includes(':eml')) {
      return 'eml';
    }

    // Check for meta.xml archive element
    if (root.localName === 'archive') {
      return 'meta';
    }

    return 'raw';
  } catch (_e) {
    return 'raw';
  }
}

const processedFiles = $derived<ProcessedXmlFile[]>(
  metadata?.xml_files
    .map((file) => {
      const type = detectXmlType(file.content);
      const processed: ProcessedXmlFile = {
        filename: file.filename,
        content: file.content,
        type,
      };

      if (type === 'eml') {
        processed.emlData = parseEML(file.content);
      } else if (type === 'meta') {
        processed.metaData = parseMeta(file.content);
      }

      return processed;
    })
    .toSorted((a, b) => {
      if (a.type < b.type) return -1;
      if (b.type < a.type) return 1;
      return 0;
    }) ?? [],
);

$effect(() => {
  if (processedFiles && processedFiles.length > 0) {
    const emlFilename = processedFiles.find((f) => f.type === 'eml')?.filename;
    if (emlFilename) activeTab = emlFilename;
  }
});

async function loadMetadata() {
  try {
    loading = true;
    error = null;
    metadata = await invoke<ArchiveMetadata>('get_archive_metadata');
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    console.error('Failed to load metadata:', e);
  } finally {
    loading = false;
  }
}

// Set window title and initialize filtered total when archive loads
$effect(() => {
  if (archive) {
    getCurrentWindow().setTitle(`${archive.name} – metadata`);
  }
});

onMount(() => {
  loadMetadata();
  invoke('current_archive')
    .then((result) => {
      archive = result as ArchiveInfo;
    })
    .catch((_e) => {
      // it's ok if there's no open archive
    });
});

function displayType(fileType: string) {
  if (fileType === 'eml') return 'EML';
  if (fileType === 'meta') return 'Metafile';
  return 'Unknown';
}
</script>

<div class="w-full h-screen flex flex-col p-6">
  <div class="flex flex-row items-center mb-6 justify-between">
    <h1 class="text-2xl font-bold">Archive Metadata</h1>
    <Switch checked={viewingSource} onCheckedChange={() => viewingSource = !viewingSource}>
      <Switch.Control>
        <Switch.Thumb />
      </Switch.Control>
      <Switch.Label>View Source</Switch.Label>
      <Switch.HiddenInput />
    </Switch>

  </div>

  {#if loading}
    <div class="flex items-center justify-center flex-1">
      <p class="text-surface-600-400">Loading metadata...</p>
    </div>
  {:else if error}
    <div class="flex items-center justify-center flex-1">
      <div class="bg-red-100 dark:bg-red-900/20 border border-red-400 text-red-700 dark:text-red-400 px-4 py-3 rounded">
        <p class="font-bold">Error loading metadata</p>
        <p>{error}</p>
      </div>
    </div>
  {:else if metadata && processedFiles.length > 0}
    <Tabs
      value={activeTab}
      onValueChange={(details) => (activeTab = details.value)}
      class="flex h-full flex-col overflow-hidden"
    >
      <Tabs.List class="border-b border-surface-200-800 mb-4">
        {#each processedFiles as file}
          <Tabs.Trigger value={file.filename} class="btn hover:preset-tonal">
            {displayType(file.type)} ({file.filename})
          </Tabs.Trigger>
        {/each}
        <Tabs.Indicator class="bg-surface-950-50" />
      </Tabs.List>

      {#each processedFiles as file}
        <Tabs.Content value={file.filename} class="flex-1 overflow-auto">
          {#if viewingSource}
            <pre class="overflow-auto text-xs">{prettify(file.content)}</pre>
          {:else}
            {#if file.type === 'eml' && file.emlData}
              <EMLDisplay data={file.emlData} />
            {:else if file.type === 'meta' && file.metaData}
              <MetaDisplay data={file.metaData} />
            {:else}
              <div class="preset-filled-error-50-950 rounded p-4">
                Failed to make this pretty. Here's the source anyway.
              </div>
              <pre class="overflow-auto text-xs">{prettify(file.content)}</pre>
            {/if}
          {/if}
        </Tabs.Content>
      {/each}
    </Tabs>
  {:else}
    <div class="flex items-center justify-center flex-1">
      <p class="text-surface-600-400 italic">No metadata files found in this archive</p>
    </div>
  {/if}
</div>
