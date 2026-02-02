import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for testing the Chuck desktop application.
 * Since this is a Tauri app, we test against the Vite dev server with
 * mocked Tauri APIs rather than the compiled desktop app.
 *
 * Using WebKit since Tauri uses the system's native webview (Safari/WebKit on macOS).
 */
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
  },

  projects: [
    {
      name: 'integration',
      // Note that Tauri uses webkit on Mac and Linux:
      // https://github.com/tauri-apps/wry?tab=readme-ov-file#platform-considerations
      use: { ...devices['Desktop Safari'] },
      testIgnore: '**/performance.spec.ts',
    },
    {
      name: 'performance',
      use: { ...devices['Desktop Safari'] },
      testMatch: '**/performance.spec.ts',
    },
    {
      name: 'integration-windows',
      use: {
        ...devices['Desktop Edge'],
        channel: 'msedge'
      },
      testIgnore: '**/performance.spec.ts',
    },
  ],

  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
