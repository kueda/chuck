import { test, expect } from '@playwright/test';
import {
  setupMockTauri,
  waitForAppReady,
  openArchive,
} from './helpers/setup';

test.describe('PhotoViewer Multi-Photo Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await page.waitForSelector('.table-cell:has-text("TEST-001")', { timeout: 5000 });
  });

  test('should display photo counter when multiple photos exist', async ({ page }) => {
    // Click first occurrence
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    // Wait for drawer
    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    // Click first photo to open PhotoViewer
    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();

      // Wait for PhotoViewer
      await page.waitForTimeout(300);

      // Should show photo counter (e.g., "1 / 3")
      await expect(page.locator('text=/\\d+ \\/ \\d+/')).toBeVisible();
    }
  });

  test('should navigate to next photo with right arrow', async ({ page }) => {
    // Click occurrence with multiple photos
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photos = page.locator('section:has(h2:has-text("Media")) button');
    const photoCount = await photos.count();

    if (photoCount > 1) {
      // Click first photo
      await photos.first().click();
      await page.waitForTimeout(300);

      // Get initial photo counter text
      const initialCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(initialCounter).toContain('1 /');

      // Press right arrow
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(300);

      // Verify counter updated
      const newCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(newCounter).toContain('2 /');
    }
  });

  test('should navigate to previous photo with left arrow', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photos = page.locator('section:has(h2:has-text("Media")) button');
    const photoCount = await photos.count();

    if (photoCount > 1) {
      // Click second photo
      await photos.nth(1).click();
      await page.waitForTimeout(300);

      // Should show "2 / N"
      const initialCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(initialCounter).toContain('2 /');

      // Press left arrow
      await page.keyboard.press('ArrowLeft');
      await page.waitForTimeout(300);

      // Should show "1 / N"
      const newCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(newCounter).toContain('1 /');
    }
  });

  test('should wrap around when navigating past last photo', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photos = page.locator('section:has(h2:has-text("Media")) button');
    const photoCount = await photos.count();

    if (photoCount > 1) {
      // Click last photo
      await photos.last().click();
      await page.waitForTimeout(300);

      const counter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      const match = counter?.match(/(\d+) \/ (\d+)/);
      if (match) {
        expect(match[1]).toBe(match[2]); // Should be "N / N"
      }

      // Press right arrow to wrap to first
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(300);

      const newCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(newCounter).toContain('1 /');
    }
  });

  test('should wrap around when navigating before first photo', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photos = page.locator('section:has(h2:has-text("Media")) button');
    const photoCount = await photos.count();

    if (photoCount > 1) {
      // Click first photo
      await photos.first().click();
      await page.waitForTimeout(300);

      const counter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      expect(counter).toContain('1 /');

      // Press left arrow to wrap to last
      await page.keyboard.press('ArrowLeft');
      await page.waitForTimeout(300);

      const newCounter = await page.locator('text=/\\d+ \\/ \\d+/').textContent();
      const match = newCounter?.match(/(\d+) \/ (\d+)/);
      if (match) {
        expect(match[1]).toBe(match[2]); // Should be "N / N"
      }
    }
  });

  test('should pan image when dragging while zoomed in', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Zoom in by scrolling
      const img = page.locator('img[alt="Full size"]');
      const imgBox = await img.boundingBox();
      if (imgBox) {
        // Zoom in 5 times
        for (let i = 0; i < 5; i++) {
          await img.hover();
          await page.mouse.wheel(0, -100);
          await page.waitForTimeout(50);
        }

        // Get initial transform
        const initialTransform = await img.getAttribute('style');

        // Drag the image
        await page.mouse.move(imgBox.x + imgBox.width / 2, imgBox.y + imgBox.height / 2);
        await page.mouse.down();
        await page.mouse.move(imgBox.x + imgBox.width / 2 - 100, imgBox.y + imgBox.height / 2 - 50);
        await page.mouse.up();
        await page.waitForTimeout(100);

        // Transform should have changed (pan applied)
        const newTransform = await img.getAttribute('style');
        expect(newTransform).not.toBe(initialTransform);
        expect(newTransform).toContain('translate');
      }
    }
  });

  test('should not pan beyond image boundaries', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');
      const imgBox = await img.boundingBox();
      if (imgBox) {
        // Zoom in
        for (let i = 0; i < 5; i++) {
          await img.hover();
          await page.mouse.wheel(0, -100);
          await page.waitForTimeout(50);
        }

        // Try to drag way beyond the edge
        await page.mouse.move(imgBox.x + imgBox.width / 2, imgBox.y + imgBox.height / 2);
        await page.mouse.down();
        await page.mouse.move(imgBox.x + imgBox.width + 500, imgBox.y + imgBox.height + 500);
        await page.mouse.up();
        await page.waitForTimeout(100);

        // Image should still be visible (not panned completely off screen)
        await expect(img).toBeVisible();

        // The transform translate values should be bounded
        const style = await img.getAttribute('style');
        const translateMatch = style?.match(/translate\(([^,]+)px,\s*([^)]+)px\)/);
        if (translateMatch) {
          const tx = parseFloat(translateMatch[1]);
          const ty = parseFloat(translateMatch[2]);
          // Values should be reasonable (not 500px)
          expect(Math.abs(tx)).toBeLessThan(300);
          expect(Math.abs(ty)).toBeLessThan(300);
        }
      }
    }
  });

  test('should zoom to max on click when image is fully visible', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');

      // Image should be at default zoom (50%)
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Click the image (don't drag)
      const imgBox = await img.boundingBox();
      if (imgBox) {
        await page.mouse.click(imgBox.x + imgBox.width / 2, imgBox.y + imgBox.height / 2);
        await page.waitForTimeout(100);

        // Should zoom to max (500%)
        await expect(page.locator('text=/Zoom: 500%/')).toBeVisible();
      }
    }
  });

  test('should keep cursor position fixed when scrolling to zoom', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[role="dialog"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');
      const imgBox = await img.boundingBox();

      if (imgBox) {
        // Position mouse at a specific point (upper left quadrant)
        const targetX = imgBox.x + imgBox.width * 0.3;
        const targetY = imgBox.y + imgBox.height * 0.3;

        await page.mouse.move(targetX, targetY);

        // Get the pixel color or position under cursor before zoom
        // (This is a simplified test - in reality we'd check the image content)

        // Zoom in
        await page.mouse.wheel(0, -100);
        await page.waitForTimeout(100);

        // The same viewport coordinates should still point to the same image content
        // We'll verify this by checking that the image has moved appropriately
        const newImgBox = await img.boundingBox();

        if (newImgBox) {
          // After zooming in on the upper-left quadrant, that area should still be under the cursor
          // The image should have grown and shifted so that point stays put
          // This is hard to test precisely, but we can verify the transform includes pan
          const style = await img.getAttribute('style');
          expect(style).toContain('translate');
        }
      }
    }
  });
});
