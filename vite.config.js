import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), sveltekit()],
  // Vite options tailored for Tauri development and only applied in `tauri
  // dev` or `tauri build`
  //
  // Prevent Vite from obscuring rust errors
  clearScreen: false,
  // Tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    // Ignore backend code and common large files to prevent Vite from
    // watching irrelevant files (function-based ignored doesn't work in Vite 6)
    watch: {
      ignored: [
        '**/src-tauri/**',
        '**/chuck-core/**',
        '**/chuck-cli/**',
        '**/target/**',
        '**/*.dmg',
        '**/*.iso',
        '**/*.zip',
        '**/*.tar',
        '**/*.tar.gz',
        '**/*.tgz',
        '**/*.rar',
        '**/*.7z',
        '**/*.db',
        '**/*.sqlite',
        '**/*.sqlite3',
      ],
    },
  },

  test: {
    // For unit tests w/ vitest, we want to ignore the playwright integration
    // tests in tests/
    exclude: ['**/node_modules/**', '**/tests/**'],

    // and we apparently need to specify an environment
    environment: 'jsdom',
  },
}));
