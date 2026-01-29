import { expect, test } from '@playwright/test';
import {
  openArchive,
  setupMockTauri,
  switchToView,
  waitForAppReady,
} from './helpers/setup';

test.describe('OccurrenceDrawer window resize', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await page.waitForSelector('.table-cell:has-text("TEST-001")', {
      timeout: 5000,
    });
  });

  test('should keep drawer open when window resizes in cards view', async ({
    page,
  }) => {
    // Switch to cards view
    await switchToView(page, 'cards');
    await page.waitForTimeout(300);

    // Click a card to open the drawer
    const firstCard = page.locator('main button:has(.occurrence-card)').first();
    await firstCard.click();

    // Wait for drawer to open
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Verify drawer is visible
    await expect(
      page.locator('[data-testid="occurrence-drawer"]'),
    ).toBeVisible();

    // Resize window to change number of columns (e.g., from 5 to 6 columns)
    // Default breakpoints in Cards.svelte:
    // < 1024: 3 cols, >= 1024: 4 cols, >= 1280: 5 cols, >= 1536: 6 cols
    // Let's resize from 1300 (5 cols) to 1600 (6 cols)
    await page.setViewportSize({ width: 1600, height: 900 });
    await page.waitForTimeout(500); // Wait for resize to settle

    // Verify drawer is still visible after resize
    await expect(
      page.locator('[data-testid="occurrence-drawer"]'),
    ).toBeVisible();

    // Verify drawer content is still present
    await expect(
      page.locator('header div:has-text("occurrenceID:")').first(),
    ).toBeVisible();
  });

  test('should keep drawer open when resizing changes column count multiple times', async ({
    page,
  }) => {
    // Switch to cards view
    await switchToView(page, 'cards');
    await page.waitForTimeout(300);

    // Click a card to open the drawer
    const firstCard = page.locator('main button:has(.occurrence-card)').first();
    await firstCard.click();

    // Wait for drawer to open
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Verify drawer is visible
    await expect(
      page.locator('[data-testid="occurrence-drawer"]'),
    ).toBeVisible();

    // Resize through multiple breakpoints
    const widths = [1200, 1500, 1000, 1600];
    for (const width of widths) {
      await page.setViewportSize({ width, height: 900 });
      await page.waitForTimeout(300);

      // Drawer should still be visible after each resize
      await expect(
        page.locator('[data-testid="occurrence-drawer"]'),
      ).toBeVisible();
    }
  });
});
