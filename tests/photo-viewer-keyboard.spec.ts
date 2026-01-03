import { test, expect } from '@playwright/test';
import {
  setupMockTauri,
  waitForAppReady,
  openArchive,
} from './helpers/setup';

test.describe('PhotoViewer Keyboard Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await page.waitForSelector('.table-cell:has-text("TEST-001")', { timeout: 5000 });
  });

  test('container div is keyboard focusable', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Find the container div with aria-label
      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await expect(container).toBeVisible();

      // Focus the container
      await container.focus();

      // Verify it's focused (has tabindex="0")
      const tabindex = await container.getAttribute('tabindex');
      expect(tabindex).toBe('0');
    }
  });

  test('plus key zooms in', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Initial zoom should be 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Focus container and press +
      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await container.focus();
      await page.keyboard.press('+');
      await page.waitForTimeout(100);

      // Zoom should increase (50% * 1.2 = 60%)
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();
    }
  });

  test('equals key zooms in', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Initial zoom should be 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Focus container and press =
      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await container.focus();
      await page.keyboard.press('=');
      await page.waitForTimeout(100);

      // Zoom should increase
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();
    }
  });

  test('minus key zooms out', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await container.focus();

      // Zoom in first
      await page.keyboard.press('+');
      await page.waitForTimeout(100);
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();

      // Then zoom out
      await page.keyboard.press('-');
      await page.waitForTimeout(100);

      // Should return to 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();
    }
  });

  test('zero key resets zoom', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');
      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await container.focus();

      // Zoom in several times
      for (let i = 0; i < 5; i++) {
        await page.keyboard.press('+');
        await page.waitForTimeout(50);
      }

      // Verify we're zoomed in
      const zoomText = await page.locator('text=/Zoom: \\d+%/').textContent();
      expect(zoomText).not.toContain('50%');

      // Press 0 to reset
      await page.keyboard.press('0');
      await page.waitForTimeout(100);

      // Should return to 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Pan should also be reset (check transform)
      const style = await img.getAttribute('style');
      expect(style).toContain('translate(0px, 0px)');
    }
  });

  test('zoom keys respect min limit', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await container.focus();

      // Already at min zoom (50%)
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Try to zoom out further
      await page.keyboard.press('-');
      await page.waitForTimeout(100);

      // Should stay at 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();
    }
  });

  test('zoom keys respect max limit', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const container = page.locator('[aria-label="Photo zoom and pan container"]');
      await container.focus();

      // Zoom to max (500%)
      for (let i = 0; i < 20; i++) {
        await page.keyboard.press('+');
        await page.waitForTimeout(50);
      }

      await expect(page.locator('text=/Zoom: 500%/')).toBeVisible();

      // Try to zoom in further
      await page.keyboard.press('+');
      await page.waitForTimeout(100);

      // Should stay at 500%
      await expect(page.locator('text=/Zoom: 500%/')).toBeVisible();
    }
  });

  test('arrow keys still navigate photos (regression test)', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photos = page.locator('section:has(h2:has-text("Media")) button');
    const photoCount = await photos.count();

    if (photoCount > 1) {
      // Click first photo
      await photos.first().click();
      await page.waitForTimeout(300);

      const initialCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(initialCounter).toContain('1 /');

      // Press right arrow to navigate
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(300);

      // Verify counter updated
      const newCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(newCounter).toContain('2 /');

      // Press left arrow to navigate back
      await page.keyboard.press('ArrowLeft');
      await page.waitForTimeout(300);

      const finalCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(finalCounter).toContain('1 /');
    }
  });
});
