import { expect, test } from '@playwright/test';
import {
  mockArchiveWithGbifID,
  mockSearchResultWithGbifID,
} from './fixtures/archive-data';
import { openArchive, setupMockTauri, waitForAppReady } from './helpers/setup';

test.describe('OccurrenceDrawer Keyboard Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await page.waitForSelector('.table-cell:has-text("TEST-001")', {
      timeout: 5000,
    });
  });

  test('should navigate to next occurrence with right arrow key', async ({
    page,
  }) => {
    // Click first occurrence to open drawer
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    // Wait for drawer to open
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Get initial occurrence ID from header
    const initialId = await page
      .locator('header div:has-text("occurrenceID:")')
      .textContent();

    // Press right arrow key
    await page.keyboard.press('ArrowRight');

    // Wait for occurrence to change
    await page.waitForTimeout(500);

    // Get new occurrence ID
    const newId = await page
      .locator('header div:has-text("occurrenceID:")')
      .textContent();

    // Verify occurrence changed
    expect(newId).not.toBe(initialId);
  });

  test('should navigate to previous occurrence with left arrow key', async ({
    page,
  }) => {
    // Click second occurrence to open drawer
    const secondOccurrence = page.locator('main .occurrence-item').nth(1);
    await secondOccurrence.click();

    // Wait for drawer to open
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Get initial occurrence ID
    const initialId = await page
      .locator('header div:has-text("occurrenceID:")')
      .textContent();

    // Press left arrow key
    await page.keyboard.press('ArrowLeft');

    // Wait for occurrence to change
    await page.waitForTimeout(500);

    // Get new occurrence ID
    const newId = await page
      .locator('header div:has-text("occurrenceID:")')
      .textContent();

    // Verify occurrence changed
    expect(newId).not.toBe(initialId);
  });

  test('should not trigger navigation when PhotoViewer is open', async ({
    page,
  }) => {
    // Click first occurrence
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    // Wait for drawer
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Click a photo to open PhotoViewer (if photos exist)
    const photo = page
      .locator('section:has(h2:has-text("Media")) button')
      .first();
    const hasPhotos = (await photo.count()) > 0;

    if (hasPhotos) {
      await photo.click();

      // Wait for PhotoViewer to open
      await page.waitForTimeout(300);

      // Get occurrence ID
      const occurrenceId = await page
        .locator('header div:has-text("occurrenceID:")')
        .textContent();

      // Press arrow keys
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(300);

      // Verify occurrence didn't change
      const newId = await page
        .locator('header div:has-text("occurrenceID:")')
        .textContent();
      expect(newId).toBe(occurrenceId);
    }
  });

  test('should display recordedBy attribute in drawer', async ({ page }) => {
    // Click first occurrence to open drawer
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    // Wait for drawer to open
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Verify recordedBy is visible
    await expect(
      page.locator('[role="dialog"]').getByText('Jane Smith').first(),
    ).toBeVisible();
  });

  test('should scroll to occurrence when navigating via next button', async ({
    page,
  }) => {
    // Click first occurrence to open drawer
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Click Next button multiple times to navigate far enough that we need to scroll
    const nextButton = page.locator('button:has-text("Next")');
    for (let i = 0; i < 10; i++) {
      await nextButton.click();
      await page.waitForTimeout(100);
    }

    // Get the currently selected occurrence (should be at index 10)
    const occurrenceAtIndex10 = page.locator('main .occurrence-item').nth(10);

    // Check if the occurrence is in the viewport
    // If scrolling works, it should be visible
    await expect(occurrenceAtIndex10).toBeInViewport();
  });

  test('should scroll to occurrence when navigating via previous button', async ({
    page,
  }) => {
    // Scroll down to a later occurrence
    const mainElement = page.locator('main');
    await mainElement.evaluate((el) => {
      el.scrollTop = 2000; // Scroll down significantly
    });
    await page.waitForTimeout(300);

    // Click an occurrence that's now in view (somewhere in the middle of the list)
    const middleOccurrence = page.locator('main .occurrence-item').nth(30);
    await middleOccurrence.click();
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Click Previous button multiple times to navigate back up
    const prevButton = page.locator('button:has-text("Prev")');
    for (let i = 0; i < 10; i++) {
      await prevButton.click();
      await page.waitForTimeout(100);
    }

    // The occurrence at index 20 should now be scrolled into view
    const occurrenceAtIndex20 = page.locator('main .occurrence-item').nth(20);
    await expect(occurrenceAtIndex20).toBeInViewport();
  });
});

test.describe('OccurrenceDrawer with non-occurrenceID core columns', () => {
  test.beforeEach(async ({ page }) => {
    // Setup with gbifID archive
    await setupMockTauri(page, {
      archive: mockArchiveWithGbifID,
      searchResult: mockSearchResultWithGbifID,
    });
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await page.waitForSelector('.table-cell:has-text("GBIF-001")', {
      timeout: 5000,
    });
  });

  test('should show occurrence details when clicking row in gbifID archive', async ({
    page,
  }) => {
    // Verify table header shows gbifID column
    await expect(
      page.locator('.table-header-cell:has-text("gbifID")'),
    ).toBeVisible();

    // Click first occurrence row
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    // Wait for drawer to open
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Verify drawer shows the correct occurrence details
    await expect(
      page.locator('header div:has-text("gbifID:")').first(),
    ).toBeVisible();
    await expect(
      page.locator('[role="dialog"]').getByText('GBIF-001').first(),
    ).toBeVisible();
    await expect(
      page.locator('[role="dialog"]').getByText('Lynx rufus').first(),
    ).toBeVisible();
  });

  test('should navigate between occurrences in gbifID archive', async ({
    page,
  }) => {
    // Click first occurrence
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();
    await page.waitForSelector('[data-testid="occurrence-drawer"]', {
      timeout: 5000,
    });

    // Verify first occurrence
    await expect(
      page.locator('[role="dialog"]').getByText('GBIF-001').first(),
    ).toBeVisible();

    // Click Next button
    const nextButton = page.locator('button:has-text("Next")');
    await nextButton.click();
    await page.waitForTimeout(500);

    // Verify second occurrence
    await expect(
      page.locator('[role="dialog"]').getByText('GBIF-002').first(),
    ).toBeVisible();
    await expect(
      page.locator('[role="dialog"]').getByText('Odocoileus hemionus').first(),
    ).toBeVisible();
  });
});
