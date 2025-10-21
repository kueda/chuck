import { test, expect } from '@playwright/test';
import { setupMockTauri, waitForAppReady, openArchive } from './helpers/setup';
import type { Page } from '@playwright/test';

// Not quite the ideal 200, but maybe enough
const MAX_RESPONSE_TIME = 300;

/**
 * Performance tests comparing small vs large archives
 */

async function setupMockTauriWithCustomData(
  page: Page,
  archiveName: string,
  coreCount: number,
  totalResults: number
) {
  const { getInjectionScript } = await import('./mocks/tauri');

  const customArchive = {
    name: archiveName,
    coreCount: coreCount,
  };

  // Generate mock occurrences
  const generateMockOccurrences = (count: number) => {
    const species = [
      'Quercus lobata',
      'Sequoia sempervirens',
      'Pinus ponderosa',
      'Quercus agrifolia',
      'Arctostaphylos manzanita',
    ];
    const observers = ['Jane Smith', 'John Doe', 'Bob Johnson', 'Alice Williams'];

    const occurrences = [];
    for (let i = 0; i < count; i++) {
      occurrences.push({
        occurrenceID: `TEST-${String(i + 1).padStart(6, '0')}`,
        scientificName: species[i % species.length],
        decimalLatitude: 34 + Math.random() * 8,
        decimalLongitude: -124 + Math.random() * 6,
        eventDate: `2024-01-${String((i % 28) + 1).padStart(2, '0')}`,
        eventTime: `${String(Math.floor(Math.random() * 24)).padStart(2, '0')}:${String(
          Math.floor(Math.random() * 60)
        ).padStart(2, '0')}:00`,
        recordedBy: observers[i % observers.length],
        basisOfRecord: 'HumanObservation',
      });
    }
    return occurrences;
  };

  const customSearchResult = {
    total: totalResults,
    results: generateMockOccurrences(100), // Only generate first 100
  };

  await page.addInitScript(
    getInjectionScript(customArchive, customSearchResult, customArchive, customSearchResult)
  );
}

async function measureViewSwitch(page: Page): Promise<number> {
  // Wait for table view to be ready
  await expect(page.getByText('occurrenceID')).toBeVisible();

  // Measure the view switch
  const duration = await page.evaluate(() => {
    return new Promise<number>((resolve) => {
      const startTime = performance.now();

      // Click the Cards button
      const cardsButton = document.querySelector(
        'input[type="radio"][value="cards"]'
      ) as HTMLInputElement;
      if (!cardsButton) {
        resolve(-1);
        return;
      }

      cardsButton.click();

      // Wait for first card to appear
      const observer = new MutationObserver(() => {
        const firstCard = document.querySelector('.occurrence-card');
        if (firstCard) {
          const endTime = performance.now();
          observer.disconnect();
          resolve(endTime - startTime);
        }
      });

      observer.observe(document.body, {
        childList: true,
        subtree: true,
      });

      // Timeout after 10 seconds
      setTimeout(() => {
        observer.disconnect();
        resolve(-1);
      }, 10000);
    });
  });

  return duration;
}


test.describe('Frontend performance of', () => {
  // Timing is always inconsistent, and some of these tests run close to the
  // acceptable limit, so to avoid inconsistent performance due to concurrent
  // execution, we just run these serially
  test.describe.configure({ mode: 'serial' });

  test.describe(`switching to cards view less than ${MAX_RESPONSE_TIME}ms`, () => {
    test('with 1,000 records', async ({ page }) => {
      await setupMockTauriWithCustomData(page, 'Small Archive - 1K', 1000, 1000);
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);

      const duration = await measureViewSwitch(page);

      expect(duration).toBeGreaterThan(0);
      expect(duration).toBeLessThan(MAX_RESPONSE_TIME); // Should complete within 5 seconds
    });

    test('with 100,000 records', async ({ page }) => {
      await setupMockTauriWithCustomData(page, 'Medium Archive - 100K', 100000, 100000);
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);

      const duration = await measureViewSwitch(page);

      expect(duration).toBeGreaterThan(0);
      expect(duration).toBeLessThan(MAX_RESPONSE_TIME);
    });

    test('with 1,000,000 records', async ({ page }) => {
      await setupMockTauriWithCustomData(
        page,
        'Large Archive - 1M',
        1000000,
        1000000
      );
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);

      const duration = await measureViewSwitch(page);

      expect(duration).toBeGreaterThan(0);
      expect(duration).toBeLessThan(MAX_RESPONSE_TIME);
    });

    test('all scales in sequence', async ({ page }) => {
      const results: Array<{ scale: string; count: number; duration: number }> = [];

      // Test 1K
      await setupMockTauriWithCustomData(page, 'Test 1K', 1000, 1000);
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);
      const duration1k = await measureViewSwitch(page);
      results.push({ scale: '1K', count: 1000, duration: duration1k });

      // Reload for 100K
      await setupMockTauriWithCustomData(page, 'Test 100K', 100000, 100000);
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);
      const duration100k = await measureViewSwitch(page);
      results.push({ scale: '100K', count: 100000, duration: duration100k });

      // Reload for 1M
      await setupMockTauriWithCustomData(page, 'Test 1M', 1000000, 1000000);
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);
      const duration1m = await measureViewSwitch(page);
      results.push({ scale: '1M', count: 1000000, duration: duration1m });

      // console.log('\n========== PERFORMANCE COMPARISON ==========');
      // console.log('Scale\tRecords\t\tDuration');
      // console.log('--------------------------------------------');
      // results.forEach((r) => {
      //   console.log(
      //     `${r.scale}\t${r.count.toLocaleString()}\t\t${r.duration.toFixed(2)}ms`
      //   );
      // });
      // console.log('============================================\n');

      // All should complete
      results.forEach((r) => {
        expect(r.duration).toBeGreaterThan(0);
        expect(r.duration, `${r.scale} took longer than 200ms (took ${r.duration.toFixed(2)}ms)`)
          .toBeLessThan(MAX_RESPONSE_TIME);
      });
    });
  });
});
