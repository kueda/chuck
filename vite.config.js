import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), sveltekit()],
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // 3. tell Vite to ignore watching `src-tauri`
    watch: {
      ignored: ["**/src-tauri/**", "**/chuck-core/**", "**/chuck-cli/**"]
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
