import { test, expect } from '@playwright/test';
import { setupMockTauri, waitForAppReady, openArchive } from './helpers/setup';

test.describe('Groups Tab - Count Click', () => {
  test.beforeEach(async ({ page }) => {
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
  });

  test('clicking a group count should filter Occurrences tab and show filter in sidebar', async ({ page }) => {
    await openArchive(page);

    // Switch to Groups tab
    const groupsTab = page.locator('button:has-text("Groups")');
    await groupsTab.click();
    await page.waitForTimeout(500);

    // Verify we're on Groups tab
    await expect(page.getByText('Group by:')).toBeVisible();

    // Verify default grouping is by scientificName
    const groupBySelect = page.locator('select#group-by-field');
    await expect(groupBySelect).toHaveValue('scientificName');

    // Wait for aggregation results to load
    await page.waitForTimeout(1000);

    // Get the second row in the table (first data row after header)
    const secondRow = page.locator('.table tbody tr').nth(1);
    await expect(secondRow).toBeVisible();

    // Get the scientific name value from the second row
    const scientificNameCell = secondRow.locator('td').first();
    const scientificNameValue = await scientificNameCell.textContent();

    // Click the count button in the second row
    const countButton = secondRow.locator('button');
    await countButton.click();

    // Wait for tab switch
    await page.waitForTimeout(1000);

    // Verify we're now on Occurrences tab (use same locator pattern as Groups tab)
    const occTab = page.locator('button:has-text("Occurrences")');
    await expect(occTab).toBeVisible();

    // Check if occurrences tab is active by seeing if Occurrences content is visible
    const occTabContent = page.locator('[data-testid="occurrences-scroll-container"]');
    await expect(occTabContent).toBeVisible();

    // Wait a bit more for the filters to sync
    await page.waitForTimeout(1000);

    // Verify the first occurrence row shows the expected scientific name
    const firstOccurrenceRow = page.locator('.occurrence-row').first();
    await expect(firstOccurrenceRow).toBeVisible();

    // The scientific name is in the 3rd column (index 2)
    const firstRowScientificName = await firstOccurrenceRow.locator('.table-cell').nth(2).textContent();
    expect(firstRowScientificName).toBe(scientificNameValue);

    // Open the Taxonomy section to check filters
    const taxonomySection = page.locator('#Filters').getByRole('button').filter({ hasText: 'Taxonomy' });
    await expect(taxonomySection).toBeVisible();
    const taxonomySectionText = await taxonomySection.textContent();
    await taxonomySection.click();
    await page.waitForTimeout(500);

    // Verify the scientificName filter input has the expected value
    const scientificNameInput = page.getByRole('combobox', { name: 'scientificName' })
    await expect(scientificNameInput).toBeVisible();
    const inputValue = await scientificNameInput.inputValue();
    expect(inputValue).toBe(scientificNameValue);
  });
});
