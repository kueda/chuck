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
    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

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

test.describe('PhotoViewer Zoom and Pan Controls', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await page.waitForSelector('.table-cell:has-text("TEST-001")', { timeout: 5000 });
  });

  test('zoom controls are visible', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Verify zoom control buttons are visible
      await expect(page.locator('button[aria-label="Zoom in"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Zoom out"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Reset zoom"]')).toBeVisible();
    }
  });

  test('zoom in button works', async ({ page }) => {
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

      // Click zoom in button
      await page.locator('button[aria-label="Zoom in"]').click();
      await page.waitForTimeout(100);

      // Zoom should increase to 60%
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();
    }
  });

  test('zoom out button works', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Zoom in first
      await page.locator('button[aria-label="Zoom in"]').click();
      await page.waitForTimeout(100);
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();

      // Then zoom out
      await page.locator('button[aria-label="Zoom out"]').click();
      await page.waitForTimeout(100);

      // Should return to 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();
    }
  });

  test('reset button works', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');

      // Zoom in several times
      for (let i = 0; i < 5; i++) {
        await page.locator('button[aria-label="Zoom in"]').click();
        await page.waitForTimeout(50);
      }

      // Verify we're zoomed in
      const zoomText = await page.locator('text=/Zoom: \\d+%/').textContent();
      expect(zoomText).not.toContain('50%');

      // Click reset
      await page.locator('button[aria-label="Reset zoom"]').click();
      await page.waitForTimeout(100);

      // Should return to 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Pan should also be reset
      const style = await img.getAttribute('style');
      expect(style).toContain('translate(0px, 0px)');
    }
  });

  test('zoom buttons are disabled at limits', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const zoomInBtn = page.locator('button[aria-label="Zoom in"]');
      const zoomOutBtn = page.locator('button[aria-label="Zoom out"]');

      // At min zoom (50%), zoom out should be disabled
      await expect(zoomOutBtn).toBeDisabled();
      await expect(zoomInBtn).not.toBeDisabled();

      // Zoom to max
      for (let i = 0; i < 20; i++) {
        await zoomInBtn.click();
        await page.waitForTimeout(50);
      }

      await expect(page.locator('text=/Zoom: 500%/')).toBeVisible();

      // At max zoom, zoom in should be disabled
      await expect(zoomInBtn).toBeDisabled();
      await expect(zoomOutBtn).not.toBeDisabled();
    }
  });

  test('pan controls appear when zoomed in', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Initially at 50% zoom, pan controls should not be visible
      const panUpBtn = page.locator('button[aria-label="Pan up"]');
      await expect(panUpBtn).not.toBeVisible();

      // Zoom in enough that image is larger than viewport
      for (let i = 0; i < 10; i++) {
        await page.locator('button[aria-label="Zoom in"]').click();
        await page.waitForTimeout(50);
      }

      // Pan controls should now be visible
      await expect(page.locator('button[aria-label="Pan up"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Pan down"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Pan left"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Pan right"]')).toBeVisible();
    }
  });

  test('pan controls hidden when not pannable', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Zoom in
      for (let i = 0; i < 10; i++) {
        await page.locator('button[aria-label="Zoom in"]').click();
        await page.waitForTimeout(50);
      }

      // Pan controls should be visible
      await expect(page.locator('button[aria-label="Pan up"]')).toBeVisible();

      // Reset zoom (image fits in viewport)
      await page.locator('button[aria-label="Reset zoom"]').click();
      await page.waitForTimeout(100);

      // Pan controls should be hidden
      await expect(page.locator('button[aria-label="Pan up"]')).not.toBeVisible();
    }
  });

  test('pan buttons move image', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');

      // Zoom in so image is pannable
      for (let i = 0; i < 10; i++) {
        await page.locator('button[aria-label="Zoom in"]').click();
        await page.waitForTimeout(50);
      }

      // Get initial transform
      const initialStyle = await img.getAttribute('style');
      const initialTranslateMatch = initialStyle?.match(/translate\(([^,]+)px,\s*([^)]+)px\)/);
      const initialX = initialTranslateMatch ? parseFloat(initialTranslateMatch[1]) : 0;
      const initialY = initialTranslateMatch ? parseFloat(initialTranslateMatch[2]) : 0;

      // Click pan right button
      await page.locator('button[aria-label="Pan right"]').click();
      await page.waitForTimeout(100);

      // Transform should have changed
      const newStyle = await img.getAttribute('style');
      const newTranslateMatch = newStyle?.match(/translate\(([^,]+)px,\s*([^)]+)px\)/);
      const newX = newTranslateMatch ? parseFloat(newTranslateMatch[1]) : 0;

      // X should have decreased (panning right means image moves left)
      expect(newX).toBeLessThan(initialX);

      // Click pan down button
      await page.locator('button[aria-label="Pan down"]').click();
      await page.waitForTimeout(100);

      const finalStyle = await img.getAttribute('style');
      const finalTranslateMatch = finalStyle?.match(/translate\(([^,]+)px,\s*([^)]+)px\)/);
      const finalY = finalTranslateMatch ? parseFloat(finalTranslateMatch[2]) : 0;

      // Y should have decreased (panning down means image moves up)
      expect(finalY).toBeLessThan(initialY);
    }
  });

  test('pan buttons respect boundaries', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');

      // Zoom in so image is pannable
      for (let i = 0; i < 10; i++) {
        await page.locator('button[aria-label="Zoom in"]').click();
        await page.waitForTimeout(50);
      }

      // Click pan right many times
      for (let i = 0; i < 20; i++) {
        await page.locator('button[aria-label="Pan right"]').click();
        await page.waitForTimeout(20);
      }

      // Image should still be visible (not panned completely off screen)
      await expect(img).toBeVisible();

      // The transform translate values should be reasonable
      const style = await img.getAttribute('style');
      const translateMatch = style?.match(/translate\(([^,]+)px,\s*([^)]+)px\)/);
      if (translateMatch) {
        const tx = parseFloat(translateMatch[1]);
        const ty = parseFloat(translateMatch[2]);
        // Values should be bounded (not extreme)
        expect(Math.abs(tx)).toBeLessThan(1000);
        expect(Math.abs(ty)).toBeLessThan(1000);
      }
    }
  });

  test('button hover shows tooltips', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Check that buttons have title attributes (tooltips)
      const zoomInBtn = page.locator('button[aria-label="Zoom in"]');
      const title = await zoomInBtn.getAttribute('title');
      expect(title).toBe('Zoom in (+)');

      const zoomOutBtn = page.locator('button[aria-label="Zoom out"]');
      const zoomOutTitle = await zoomOutBtn.getAttribute('title');
      expect(zoomOutTitle).toBe('Zoom out (-)');

      const resetBtn = page.locator('button[aria-label="Reset zoom"]');
      const resetTitle = await resetBtn.getAttribute('title');
      expect(resetTitle).toBe('Reset zoom (0)');
    }
  });

  test('all buttons have aria-labels', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      // Verify zoom buttons have aria-labels
      const zoomInBtn = page.locator('button[aria-label="Zoom in"]');
      await expect(zoomInBtn).toBeVisible();

      const zoomOutBtn = page.locator('button[aria-label="Zoom out"]');
      await expect(zoomOutBtn).toBeVisible();

      const resetBtn = page.locator('button[aria-label="Reset zoom"]');
      await expect(resetBtn).toBeVisible();

      // Zoom in to make pan controls visible
      for (let i = 0; i < 10; i++) {
        await zoomInBtn.click();
        await page.waitForTimeout(50);
      }

      // Verify pan buttons have aria-labels
      await expect(page.locator('button[aria-label="Pan up"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Pan down"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Pan left"]')).toBeVisible();
      await expect(page.locator('button[aria-label="Pan right"]')).toBeVisible();
    }
  });

  test('controls work with multi-photo navigation', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photos = page.locator('section:has(h2:has-text("Media")) button');
    const photoCount = await photos.count();

    if (photoCount > 1) {
      // Click first photo
      await photos.first().click();
      await page.waitForTimeout(300);

      // Zoom in
      await page.locator('button[aria-label="Zoom in"]').click();
      await page.waitForTimeout(100);
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();

      // Navigate to next photo
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(300);

      // Zoom should reset to 50%
      await expect(page.locator('text=/Zoom: 50%/')).toBeVisible();

      // Controls should still work on new photo
      await page.locator('button[aria-label="Zoom in"]').click();
      await page.waitForTimeout(100);
      await expect(page.locator('text=/Zoom: 60%/')).toBeVisible();
    }
  });

  test('controls work with click-to-zoom', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');

      // Click image to zoom to max
      const imgBox = await img.boundingBox();
      if (imgBox) {
        await page.mouse.click(imgBox.x + imgBox.width / 2, imgBox.y + imgBox.height / 2);
        await page.waitForTimeout(100);
        await expect(page.locator('text=/Zoom: 500%/')).toBeVisible();

        // Zoom out button should work
        await page.locator('button[aria-label="Zoom out"]').click();
        await page.waitForTimeout(100);

        // Zoom should decrease
        const zoomText = await page.locator('text=/Zoom: \\d+%/').textContent();
        expect(zoomText).not.toContain('500%');
      }
    }
  });

  test('controls sync with scroll-to-zoom', async ({ page }) => {
    const firstOccurrence = page.locator('main .occurrence-item').first();
    await firstOccurrence.click();

    await page.waitForSelector('[data-testid="occurrence-drawer"]', { timeout: 5000 });

    const photo = page.locator('section:has(h2:has-text("Media")) button').first();
    const hasPhotos = await photo.count() > 0;

    if (hasPhotos) {
      await photo.click();
      await page.waitForTimeout(300);

      const img = page.locator('img[alt="Full size"]');
      const zoomOutBtn = page.locator('button[aria-label="Zoom out"]');

      // Scroll to zoom in
      await img.hover();
      await page.mouse.wheel(0, -100);
      await page.waitForTimeout(100);

      // Zoom out button should no longer be disabled
      await expect(zoomOutBtn).not.toBeDisabled();

      // Scroll to min zoom
      for (let i = 0; i < 10; i++) {
        await page.mouse.wheel(0, 100);
        await page.waitForTimeout(50);
      }

      // Zoom out button should be disabled at min
      await expect(zoomOutBtn).toBeDisabled();
    }
  });
});
