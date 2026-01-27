import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

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
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // Ignore everything other than frontend src, static files, config.
    // Otherwise if you, say, but a 33GB disk image in this dir, Vite tries
    // to watch it and maybe load it into memory and everything goes to hell
    watch: {
      ignored: (path, stats) => {
        // Always watch these directories
        if (/\/(src|static|\.svelte-kit)\//.test(path)) return false;

        // Watch config files at root
        if (/\/chuck\/[^/]+\.(js|ts|json)$/.test(path)) return false;

        // Ignore everything else
        return true;
      }
    }
  },

  test: {
    // For unit tests w/ vitest, we want to ignore the playwright integration
    // tests in tests/
    exclude: ['**/node_modules/**', '**/tests/**'],

    // and we apparently need to specify an environment
    environment: 'jsdom',
  }
}));
