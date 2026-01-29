import { expect, test } from '@playwright/test';
import {
  openArchive,
  searchByScientificName,
  setupMockTauri,
  waitForAppReady,
} from './helpers/setup';

test.describe('Groups Tab', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
  });

  test('should display Groups tab', async ({ page }) => {
    await openArchive(page);

    const groupsTab = page.locator('button:has-text("Groups")');
    await expect(groupsTab).toBeVisible();
  });

  test('should show field selector in Groups tab', async ({ page }) => {
    await openArchive(page);

    await page.click('button:has-text("Groups")');
    await expect(page.locator('label:has-text("Group by:")')).toBeVisible();
    await expect(page.locator('select#group-by-field')).toBeVisible();
  });

  test('should display aggregation results', async ({ page }) => {
    await openArchive(page);

    await page.click('button:has-text("Groups")');
    await page.selectOption('select#group-by-field', 'scientificName');

    // Wait for results table
    await expect(page.locator('table')).toBeVisible();
    await expect(page.locator('th:has-text("Field Value")')).toBeVisible();
    await expect(page.locator('th:has-text("Occurrences")')).toBeVisible();
  });

  test('should preserve selected field when switching tabs', async ({
    page,
  }) => {
    await openArchive(page);

    await page.click('button:has-text("Groups")');
    await page.selectOption('select#group-by-field', 'scientificName');

    await page.click('button:has-text("Occurrences")');
    await page.click('button:has-text("Groups")');

    const selectedValue = await page
      .locator('select#group-by-field')
      .inputValue();
    expect(selectedValue).toBe('scientificName');
  });

  test('should respect filters from Occurrences tab', async ({ page }) => {
    await openArchive(page);

    // Apply filter in Occurrences tab using the helper
    await searchByScientificName(page, 'Quercus');

    // Switch to Groups tab
    await page.click('button:has-text("Groups")');
    await page.selectOption('select#group-by-field', 'scientificName');

    // Wait for results table
    await expect(page.locator('table')).toBeVisible();

    // Verify only filtered results appear (mock should filter to Quercus matches)
    const rows = page.locator('table tbody tr');
    const rowCount = await rows.count();

    // Should have fewer results than unfiltered (which has 3 species)
    expect(rowCount).toBeLessThan(3);

    // All visible values should contain 'Quercus' or be 'None' for NULL values
    for (let i = 0; i < rowCount; i++) {
      const cellText = await rows.nth(i).locator('td').first().textContent();
      if (cellText && cellText !== 'None') {
        expect(cellText).toContain('Quercus');
      }
    }
  });
});
