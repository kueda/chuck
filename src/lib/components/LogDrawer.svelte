<script lang="ts">
import { listen } from '@tauri-apps/api/event';
import { X } from 'lucide-svelte';
import { onMount } from 'svelte';
import { saveTextFile, showSaveDialog } from '$lib/tauri-api';

interface Props {
  open: boolean;
}

let { open = $bindable() }: Props = $props();

const MAX_LINES = 500;

// Level numbers from tauri-plugin-log: 1=Error 2=Warn 3=Info 4=Debug 5=Trace
type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';

interface LogLine {
  ts: string;
  level: LogLevel;
  message: string;
}

interface LogPayload {
  level: 1 | 2 | 3 | 4 | 5;
  message: string;
}

function levelFromNumber(n: number): LogLevel {
  switch (n) {
    case 1:
      return 'error';
    case 2:
      return 'warn';
    case 3:
      return 'info';
    case 4:
      return 'debug';
    default:
      return 'trace';
  }
}

let lines = $state<LogLine[]>([]);
let filterLevel = $state<LogLevel | 'all'>('all');
let filterText = $state('');
let logContainer = $state<HTMLElement | undefined>();
let isAtBottom = $state(true);

const LEVELS: (LogLevel | 'all')[] = [
  'all',
  'error',
  'warn',
  'info',
  'debug',
  'trace',
];
const LEVEL_RANK: Record<LogLevel, number> = {
  error: 1,
  warn: 2,
  info: 3,
  debug: 4,
  trace: 5,
};

const filtered = $derived(
  lines.filter(
    (l) =>
      (filterLevel === 'all' ||
        LEVEL_RANK[l.level] <= LEVEL_RANK[filterLevel]) &&
      (filterText === '' ||
        l.message.toLowerCase().includes(filterText.toLowerCase())),
  ),
);

function levelClass(level: LogLevel) {
  switch (level) {
    case 'error':
      return 'text-red-500';
    case 'warn':
      return 'text-yellow-500';
    case 'info':
      return 'text-green-500';
    case 'debug':
      return 'text-sky-400';
    default:
      return 'text-surface-400';
  }
}

function handleScroll(e: Event) {
  const el = e.target as HTMLElement;
  isAtBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 4;
}

$effect(() => {
  if (isAtBottom && logContainer && filtered.length > 0) {
    logContainer.scrollTop = logContainer.scrollHeight;
  }
});

function formatLines(logLines: LogLine[]) {
  return logLines
    .map((l) => `[${l.ts}] ${l.level.toUpperCase().padEnd(5)} ${l.message}`)
    .join('\n');
}

function copyAll() {
  navigator.clipboard.writeText(formatLines(filtered));
}

async function exportLogs() {
  const date = new Date()
    .toISOString()
    .slice(0, 19)
    .replace('T', '_')
    .replaceAll(':', '-');
  const path = await showSaveDialog({
    defaultPath: `chuck-logs-${date}.log`,
    filters: [{ name: 'Log file', extensions: ['log', 'txt'] }],
  });
  if (!path) return;
  await saveTextFile(path as string, formatLines(filtered));
}

onMount(() => {
  const unlisten = listen<LogPayload>('log://log', (event) => {
    const { level, message } = event.payload;
    const ts = new Date().toISOString().slice(11, 23);
    const newLine: LogLine = { ts, level: levelFromNumber(level), message };
    lines =
      lines.length >= MAX_LINES
        ? [...lines.slice(-(MAX_LINES - 1)), newLine]
        : [...lines, newLine];
  });

  return () => {
    unlisten.then((fn) => fn());
  };
});
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    role="dialog"
    aria-modal="true"
    aria-label="Application Logs"
    tabindex="-1"
    class="fixed inset-0 z-50 flex flex-col bg-surface-900 text-surface-50"
    onclick={(e) => e.stopPropagation()}
  >
    <!-- Header -->
    <div class="flex items-center gap-2 px-3 py-2 border-b border-surface-700 shrink-0">
      <span class="font-semibold text-sm">Application Logs</span>
      <span class="text-xs text-surface-400">({filtered.length} lines)</span>

      <!-- Level filter -->
      <div class="flex gap-1 ml-2">
        {#each LEVELS as lvl}
          <button
            type="button"
            class="text-xs px-2 py-0.5 rounded text-yellow-500 {filterLevel === lvl
              ? 'preset-filled'
              : 'preset-tonal'}"
            onclick={() => (filterLevel = lvl as LogLevel | 'all')}
          >
            {lvl}
          </button>
        {/each}
      </div>

      <!-- Text search -->
      <input
        type="search"
        placeholder="Filter..."
        bind:value={filterText}
        class="ml-2 text-xs px-2 py-0.5 rounded bg-surface-800 border border-surface-600
               focus:outline-none focus:border-surface-400 w-40"
      />

      <div class="ml-auto flex gap-2">
        <button
          type="button"
          class="btn btn-sm preset-tonal text-xs text-yellow-500"
          onclick={exportLogs}
        >
          Export...
        </button>
        <button
          type="button"
          class="btn btn-sm preset-tonal text-xs text-yellow-500"
          onclick={copyAll}
        >
          Copy all
        </button>
        <button
          type="button"
          class="btn btn-sm preset-tonal text-yellow-500"
          aria-label="Close logs"
          onclick={() => (open = false)}
        >
          <X size={16} />
        </button>
      </div>
    </div>

    <!-- Log lines -->
    <div
      bind:this={logContainer}
      onscroll={handleScroll}
      class="flex-1 overflow-y-auto font-mono text-xs px-3 py-2 leading-5"
    >
      {#if filtered.length === 0}
        <div class="text-surface-500 italic">No log entries yet.</div>
      {:else}
        {#each filtered as line (line.ts + line.message)}
          <div class="flex gap-2 whitespace-pre-wrap break-all">
            <span class="text-surface-500 shrink-0">{line.ts}</span>
            <span class="shrink-0 w-10 {levelClass(line.level)}">{line.level.toUpperCase()}</span>
            <span>{line.message}</span>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Footer: scroll-to-bottom indicator -->
    {#if !isAtBottom && filtered.length > 0}
      <button
        type="button"
        class="absolute bottom-4 right-4 btn btn-sm preset-filled text-xs shadow-lg"
        onclick={() => {
          if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
          isAtBottom = true;
        }}
      >
        ↓ Jump to bottom
      </button>
    {/if}
  </div>
{/if}
