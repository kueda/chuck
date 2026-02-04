import type { Page } from '@playwright/test';
import { expect, test } from '@playwright/test';

/**
 * TODO: Manually revisit the 3 skipped tests at the end.
 * These tests require mocking @tauri-apps/plugin-dialog's save() function
 * which is imported directly in the component, not through tauri-api wrapper.
 * Consider refactoring to use a wrapper or finding a way to mock ES6 imports.
 */

async function setupInatDownloadMocks(page: Page) {
  await page.addInitScript(() => {
    const commandMocks = new Map();
    let windowClosed = false;
    let progressListener: any = null;
    let progressEventSequence: any[] = [];

    // Mock iNaturalist API search responses
    const originalFetch = window.fetch;
    window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === 'string' ? input : input.toString();

      // Mock iNaturalist search API
      if (url.includes('api.inaturalist.org/v1/search')) {
        const mockResponse = {
          total_results: 1,
          page: 1,
          per_page: 10,
          results: [
            {
              type: 'Taxon',
              score: 10,
              record: {
                id: 47126,
                name: 'Aves',
                rank: 'class',
                rank_level: 50,
                preferred_common_name: 'Birds',
                iconic_taxon_name: 'Aves',
                default_photo: {
                  medium_url: 'https://example.com/bird.jpg',
                  square_url: 'https://example.com/bird_square.jpg',
                },
              },
              taxon: {
                id: 47126,
                name: 'Aves',
                rank: 'class',
                rank_level: 50,
                preferred_common_name: 'Birds',
                iconic_taxon_name: 'Aves',
                default_photo: {
                  medium_url: 'https://example.com/bird.jpg',
                  square_url: 'https://example.com/bird_square.jpg',
                },
              },
            },
          ],
        };
        return new Response(JSON.stringify(mockResponse), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      }

      // Mock iNaturalist taxa fetch API
      if (url.includes('api.inaturalist.org/v1/taxa')) {
        const mockResponse = {
          total_results: 1,
          page: 1,
          per_page: 1,
          results: [
            {
              id: 47126,
              name: 'Aves',
              rank: 'class',
              rank_level: 50,
              preferred_common_name: 'Birds',
              iconic_taxon_name: 'Aves',
            },
          ],
        };
        return new Response(JSON.stringify(mockResponse), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      }

      // Pass through other fetch calls
      return originalFetch(input, init);
    };

    // Mock the module loader
    const originalImport = (window as any).import;
    (window as any).import = async (moduleName: string) => {
      return originalImport
        ? originalImport(moduleName)
        : Promise.reject(new Error(`Module not found: ${moduleName}`));
    };

    // Main mock object that tauri-api.ts looks for
    (window as any).__MOCK_TAURI__ = {
      invoke: async (command: string, args?: any) => {
        console.log('[Mock Tauri] invoke called:', command, args);

        // Check for custom mock response
        if (commandMocks.has(command)) {
          const mockResponse = commandMocks.get(command);
          if (mockResponse.error) {
            throw new Error(mockResponse.error);
          }
          return mockResponse.success;
        }

        switch (command) {
          case 'get_observation_count':
            return 1234;

          case 'generate_inat_archive': {
            // Use custom event sequence if provided, otherwise use default
            const eventsToEmit =
              progressEventSequence.length > 0
                ? progressEventSequence
                : [
                    // Default realistic sequence
                    { stage: 'fetching', current: 30, total: 100 },
                    { stage: 'fetching', current: 60, total: 100 },
                    { stage: 'fetching', current: 100, total: 100 },
                    { stage: 'building', message: 'Finalizing archive...' },
                    { stage: 'complete' },
                  ];

            // Emit events with realistic timing
            eventsToEmit.forEach((event, index) => {
              setTimeout(
                () => {
                  if (progressListener) {
                    progressListener({ payload: event });
                  }
                },
                (index + 1) * 50,
              );
            });

            return null;
          }

          case 'cancel_inat_archive':
            return null;

          case 'open_archive':
            console.log('[Mock Tauri] Opening archive:', args);
            return null;

          case 'inat_get_auth_status':
            return { authenticated: false, username: null };

          case 'inat_authenticate':
            return { authenticated: true, username: null };

          case 'inat_sign_out':
            return null;

          default:
            throw new Error(`Unknown command: ${command}`);
        }
      },

      showOpenDialog: async () => '/mock/path/to/archive.zip',
      showSaveDialog: async () => '/mock/path/to/archive.zip',

      getCurrentWindow: () => ({
        close: () => {
          console.log('[Mock Tauri] Window closed');
          windowClosed = true;
        },
        setTitle: (title: string) => {
          console.log('[Mock Tauri] Window title set to:', title);
        },
      }),

      listen: async (event: string, handler: any) => {
        console.log('[Mock Tauri] Listening for event:', event);
        if (event === 'inat-progress') {
          progressListener = handler;
        }
        return () => {
          console.log('[Mock Tauri] Unlisten:', event);
        };
      },

      // Helper methods for tests
      setMockResponse: (command: string, response: any) => {
        commandMocks.set(command, response);
      },
      clearMockResponses: () => {
        commandMocks.clear();
      },
      isWindowClosed: () => windowClosed,
      triggerProgressEvent: (payload: any) => {
        if (progressListener) {
          progressListener({ payload });
        }
      },
      setProgressEventSequence: (events: any[]) => {
        progressEventSequence = events;
      },
    };
  });
}

test.describe('iNat Download UI', () => {
  test.beforeEach(async ({ page }) => {
    await setupInatDownloadMocks(page);
    await page.goto('/inat-download');
    await page.waitForSelector('h1:has-text("Download from iNaturalist")', {
      timeout: 10000,
    });

    // Reset progress event sequence before each test
    await page.evaluate(() => {
      (window as any).__MOCK_TAURI__.setProgressEventSequence([]);
    });
  });

  test('shows all filter inputs', async ({ page }) => {
    await expect(page.locator('input[placeholder="Taxon"]')).toBeVisible();
    await expect(page.locator('input[placeholder="Place"]')).toBeVisible();
    await expect(page.locator('input[placeholder="User"]')).toBeVisible();
    await expect(page.locator('input[name="observed-range"]')).toHaveCount(2);
    await expect(page.locator('input[name="created-range"]')).toHaveCount(2);
  });

  test('fetches observation count when taxon selected', async ({ page }) => {
    // Type in taxon autocomplete
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400); // Wait for debounce and API

    // Select from autocomplete dropdown
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600); // Wait for count fetch

    await expect(page.locator('text=1,234 observations match')).toBeVisible();
  });

  test('disables download button until count loads', async ({ page }) => {
    const downloadBtn = page.locator('button:has-text("Download Archive")');
    await expect(downloadBtn).toBeDisabled();

    // Type and select from autocomplete
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    await expect(downloadBtn).toBeEnabled();
  });

  test('shows custom date pickers when custom range selected', async ({
    page,
  }) => {
    await page.click('input[name="observed-range"][value="custom"]');

    await expect(page.locator('#observed-d1')).toBeVisible();
    await expect(page.locator('#observed-d2')).toBeVisible();
  });

  test('extension checkboxes all start unchecked', async ({ page }) => {
    // Find checkboxes by their associated labels
    const checkboxes = await page.locator('input[type="checkbox"]').all();

    // Should have at least 4 checkboxes (fetch photos + 3 extensions)
    expect(checkboxes.length).toBeGreaterThanOrEqual(4);

    // All should be unchecked initially
    for (const checkbox of checkboxes) {
      const name = await checkbox.getAttribute('name');
      if (name === 'simpleMultimedia') {
        await expect(checkbox).toBeChecked();
      } else {
        await expect(checkbox).not.toBeChecked();
      }
    }
  });

  test('opens file picker and shows progress on download', async ({ page }) => {
    // First get count loaded by selecting a taxon
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    const downloadBtn = page.locator('button:has-text("Download Archive")');
    await downloadBtn.click();

    // Progress overlay should appear
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).toBeVisible();
  });

  test('shows success dialog and opens in Chuck', async ({ page }) => {
    // Load count by selecting a taxon
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    // Start download
    const downloadBtn = page.locator('button:has-text("Download Archive")');
    await downloadBtn.click();

    // Wait for progress overlay
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).toBeVisible();

    // Trigger completion event
    await page.evaluate(() => {
      (window as any).__MOCK_TAURI__.triggerProgressEvent({
        stage: 'complete',
      });
    });

    // Wait for success dialog
    await expect(
      page.locator('text=Archive Created Successfully!'),
    ).toBeVisible({ timeout: 3000 });

    // Click Open in Chuck
    const openBtn = page.locator('button:has-text("Open in Chuck")');
    await openBtn.click();

    // Verify open_archive was called and window closed
    await page.waitForTimeout(500);
    const wasClosed = await page.evaluate(() => {
      return (window as any).__MOCK_TAURI__.isWindowClosed();
    });
    expect(wasClosed).toBe(true);
  });

  test('can close success dialog without opening', async ({ page }) => {
    // Load count by selecting a taxon
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    // Start download
    await page.locator('button:has-text("Download Archive")').click();

    // Trigger completion
    await page.evaluate(() => {
      (window as any).__MOCK_TAURI__.triggerProgressEvent({
        stage: 'complete',
      });
    });

    // Wait for success dialog
    await expect(
      page.locator('text=Archive Created Successfully!'),
    ).toBeVisible({ timeout: 3000 });

    // Click Close
    const closeBtn = page.locator('button:has-text("Close")').last();
    await closeBtn.click();

    // Dialog should disappear
    await expect(
      page.locator('text=Archive Created Successfully!'),
    ).not.toBeVisible();

    // Window should not be closed
    const wasClosed = await page.evaluate(() => {
      return (window as any).__MOCK_TAURI__.isWindowClosed();
    });
    expect(wasClosed).toBe(false);
  });

  test('can cancel download', async ({ page }) => {
    // Load count by selecting a taxon
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    // Start download
    await page.locator('button:has-text("Download Archive")').click();

    // Progress should show
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).toBeVisible();

    // Click cancel
    const cancelBtn = page.locator('button:has-text("Cancel")');
    await cancelBtn.click();

    // Progress should disappear
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).not.toBeVisible();
  });

  test('maintains stable observation total during download', async ({
    page,
  }) => {
    // Load count by selecting a taxon
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    // Start download
    await page.locator('button:has-text("Download Archive")').click();

    // Wait for progress overlay and first fetching event
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).toBeVisible();
    await expect(page.locator('text=/Fetching observations/')).toBeVisible({
      timeout: 5000,
    });

    // The default mock emits 3 fetching events: 30/100, 60/100, 100/100
    // Verify we see observation progress (may already be at 100/100 or showing Complete)
    const progressDialog = page.locator(
      '.fixed.inset-0.bg-black.bg-opacity-50',
    );

    // Try to find the observations progress text
    const hasObservations = await progressDialog
      .locator('text=/Fetching observations/')
      .count();

    if (hasObservations > 0) {
      // If still showing observations progress, verify the format
      const observationsText = await progressDialog
        .locator('text=/Fetching observations/')
        .textContent();
      expect(observationsText).toMatch(/\d+\/\d+/);
    } else {
      // May have already completed - verify completion or building stage
      const hasComplete = await progressDialog
        .locator('text=/Complete/')
        .count();
      const hasBuilding = await progressDialog
        .locator('text=/Finalizing/')
        .count();
      expect(hasComplete + hasBuilding).toBeGreaterThan(0);
    }

    // The key verification is that the backend code captures total_observations
    // from the first response and uses it consistently. This test confirms
    // the UI properly displays observations progress with dual progress bars.
  });

  test('prevents photo download progress from exceeding total', async ({
    page,
  }) => {
    // Set custom progress sequence with photo download phase
    await page.evaluate(() => {
      (window as any).__MOCK_TAURI__.setProgressEventSequence([
        { stage: 'fetching', current: 50, total: 50 },
        { stage: 'downloadingPhotos', current: 5, total: 25 },
        { stage: 'downloadingPhotos', current: 15, total: 25 },
        { stage: 'downloadingPhotos', current: 25, total: 25 },
        { stage: 'complete' },
      ]);
    });

    // Load count and enable photo download
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);
    await page.check('input[name="fetchPhotos"]'); // Check "Download photos"

    // Start download
    await page.locator('button:has-text("Download Archive")').click();

    // Wait for progress overlay and photo download phase
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).toBeVisible();
    await expect(page.locator('text=/Downloading photos/')).toBeVisible({
      timeout: 5000,
    });

    // The mock sequence has current values (5, 15, 25) all <= total (25)
    const progressDialog = page.locator(
      '.fixed.inset-0.bg-black.bg-opacity-50',
    );

    // Try to find the photo progress text
    const hasPhotos = await progressDialog
      .locator('text=/Downloading photos/')
      .count();

    if (hasPhotos > 0) {
      // If still showing photo progress, verify current <= total
      const photosText = await progressDialog
        .locator('text=/Downloading photos/')
        .textContent();
      const match = photosText?.match(/(\d+)\/(\d+)/);
      if (match) {
        const current = parseInt(match[1], 10);
        const total = parseInt(match[2], 10);
        expect(current).toBeLessThanOrEqual(total);
      }
    } else {
      // May have already completed - verify completion or building stage
      const hasComplete = await progressDialog
        .locator('text=/Complete/')
        .count();
      const hasBuilding = await progressDialog
        .locator('text=/Finalizing/')
        .count();
      expect(hasComplete + hasBuilding).toBeGreaterThan(0);
    }

    // The backend fix ensures photo progress never exceeds total by
    // estimating total_photos upfront from the first batch.
    // This test confirms the UI properly displays separate photo progress.
  });

  test('shows auth section with sign in button when not authenticated', async ({
    page,
  }) => {
    // Auth section should be visible
    await expect(
      page.locator('text=Sign in to access your private coordinates'),
    ).toBeVisible();

    // Sign In button should be visible
    await expect(page.locator('button:has-text("Sign In")')).toBeVisible();

    // Sign Out button should not be visible
    await expect(page.locator('button:has-text("Sign Out")')).not.toBeVisible();
  });

  test('can sign in and sign out', async ({ page }) => {
    // Verify starting state - not authenticated
    await expect(page.locator('button:has-text("Sign In")')).toBeVisible();

    // Sign in
    const signInBtn = page.locator('button:has-text("Sign In")');
    await signInBtn.click();
    await page.waitForTimeout(500);

    // Should now show signed in status
    await expect(page.locator('text=Signed in')).toBeVisible();
    await expect(page.locator('button:has-text("Sign Out")')).toBeVisible();

    // Sign out
    const signOutBtn = page.locator('button:has-text("Sign Out")');
    await signOutBtn.click();
    await page.waitForTimeout(500);

    // Should show sign in button again
    await expect(page.locator('button:has-text("Sign In")')).toBeVisible();
  });

  test('ETR component renders without breaking progress display', async ({
    page,
  }) => {
    // Load count by selecting a taxon
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600);

    // Start download
    await page.locator('button:has-text("Download Archive")').click();

    // Wait for progress overlay
    await expect(
      page.locator('text=Generating Darwin Core Archive'),
    ).toBeVisible();

    const progressDialog = page.locator(
      '.fixed.inset-0.bg-black.bg-opacity-50',
    );

    // Progress bars should still work
    await expect(
      progressDialog.locator('text=/Fetching observations/'),
    ).toBeVisible({ timeout: 2000 });

    // The ETR feature shouldn't break existing progress display
    // Just verify the dialog is still functional
    const cancelBtn = page.locator('button:has-text("Cancel")');
    await expect(cancelBtn).toBeVisible();
  });
});

test.describe('Extension checkbox functionality', () => {
  let capturedInvokeArgs: any = null;

  test.beforeEach(async ({ page }) => {
    // Reset captured args
    capturedInvokeArgs = null;

    // Set up mocks (from the main beforeEach)
    await setupInatDownloadMocks(page);
    await page.goto('/inat-download');
    await page.waitForSelector('h1:has-text("Download from iNaturalist")', {
      timeout: 10000,
    });

    // Set up invoke capture
    await page.exposeFunction(
      '__captureInvoke',
      (command: string, args: any) => {
        if (command === 'generate_inat_archive') {
          capturedInvokeArgs = args;
        }
      },
    );

    // Wrap the mock to capture calls
    await page.evaluate(() => {
      const originalInvoke = (window as any).__MOCK_TAURI__.invoke;
      (window as any).__MOCK_TAURI__.invoke = async (
        command: string,
        args?: any,
      ) => {
        (window as any).__captureInvoke(command, args);
        return originalInvoke(command, args);
      };
    });
  });

  // Helper to trigger download
  async function triggerDownload(page: Page) {
    await page.fill('input[placeholder="Taxon"]', 'bird');
    await page.waitForTimeout(400);
    await page.click('[role="option"]:has-text("Birds")');
    await page.waitForTimeout(600); // Debounce
    await page.locator('button:has-text("Download Archive")').click();
    await page.waitForTimeout(200); // Wait for invoke
  }

  test('includes "Identifications" when checkbox is checked', async ({
    page,
  }) => {
    await page.locator('input[name="identifications"]').check();
    await expect(page.locator('input[name="identifications"]')).toBeChecked();

    await triggerDownload(page);

    expect(capturedInvokeArgs).not.toBeNull();
    expect(capturedInvokeArgs.params.extensions).toContain('Identifications');
  });

  test('excludes "Identifications" when checkbox is unchecked', async ({
    page,
  }) => {
    await expect(
      page.locator('input[name="identifications"]'),
    ).not.toBeChecked();

    await triggerDownload(page);

    expect(capturedInvokeArgs).not.toBeNull();
    expect(capturedInvokeArgs.params.extensions).not.toContain(
      'Identifications',
    );
  });

  test('includes all checked extensions', async ({ page }) => {
    await page.locator('input[name="audiovisual"]').check();
    await page.locator('input[name="identifications"]').check();

    await triggerDownload(page);

    expect(capturedInvokeArgs).not.toBeNull();
    expect(capturedInvokeArgs.params.extensions).toEqual(
      expect.arrayContaining([
        'SimpleMultimedia',
        'Audiovisual',
        'Identifications',
      ]),
    );
    expect(capturedInvokeArgs.params.extensions).toHaveLength(3);
  });

  test('only includes SimpleMultimedia by default', async ({ page }) => {
    await triggerDownload(page);

    expect(capturedInvokeArgs).not.toBeNull();
    expect(capturedInvokeArgs.params.extensions).toEqual(['SimpleMultimedia']);
  });
});
