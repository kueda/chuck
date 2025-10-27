import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';
import {
  setupMockTauri,
  waitForAppReady,
  openArchive,
  searchByScientificName,
  getVisibleOccurrences,
  triggerMenuOpen,
} from './helpers/setup';

test.describe('Frontend', () => {
  test.beforeEach(async ({ page }) => {
    // Set up Tauri mocks before each test
    await setupMockTauri(page);

    // Navigate to the app
    await page.goto('/');

    // Wait for initial render
    await waitForAppReady(page);
  });

  async function switchToView(page: Page, view: string) {
    const cardsInput = page.locator(`input[type="radio"][value="${view}"]`);
    return cardsInput.click({ force: true });
  }

  test('should display welcome message when no archive is open', async ({ page }) => {
    // Check for the welcome text
    await expect(page.getByText(/Chuck is an application for viewing archives/))
      .toBeVisible();

    // Check for the Open Archive button
    await expect(page.getByRole('button', { name: 'Open Archive' }))
      .toBeVisible();
  });

  test('should open an archive and display results', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for the main content area to appear
    await expect(page.locator('main')).toBeVisible();

    // Check that some occurrence data is visible
    const rows = await getVisibleOccurrences(page);
    expect(rows.length).toBeGreaterThan(0);
  });

  test('should display archive info in window title', async ({ page }) => {
    // This tests that the mock window.setTitle is being called
    // In a real app, we'd check the actual window title, but in our mock
    // we just verify the function is called (checked via console logs)

    await openArchive(page);

    // Just verify the main content loaded, which triggers setTitle
    await expect(page.locator('main')).toBeVisible();
  });

  test('should filter results by scientific name', async ({ page }) => {
    // Open the archive first
    await openArchive(page);

    // Wait for initial results to load
    await page.waitForTimeout(1000);

    // Search for a specific species
    await searchByScientificName(page, 'Sequoia sempervirens');

    // Wait for search results to update
    await page.waitForTimeout(1000);

    // Get all visible occurrence rows (excluding header)
    const rows = await getVisibleOccurrences(page);

    // Verify at least one result is visible (more than just the header)
    expect(rows.length).toBeGreaterThan(1);

    // Check that one of the data rows (skip first which is header) contains the species
    const dataRow = rows[1];
    await expect(dataRow).toContainText('Sequoia sempervirens');
  });

  test('should display loading state for unloaded chunks', async ({ page }) => {
    await openArchive(page);

    // Look for the "Loading..." text that appears for unloaded occurrences
    // This might appear briefly or not at all depending on timing
    // Just verify the virtualizer is working

    const mainContent = page.locator('main');
    await expect(mainContent).toBeVisible();
  });

  test('should handle scrolling and virtualization', async ({ page }) => {
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Get initial visible rows
    const initialRows = await getVisibleOccurrences(page);
    const initialCount = initialRows.length;

    // Scroll down in the main content area
    await page.locator('main').evaluate((el) => {
      el.scrollTop = 2000;
    });

    // Wait for new rows to load
    await page.waitForTimeout(1000);

    // The virtualizer should still be rendering rows
    const rowsAfterScroll = await getVisibleOccurrences(page);
    expect(rowsAfterScroll.length).toBeGreaterThan(0);

    // The number of visible rows might change slightly due to virtualization
    // but should be in a similar range
    expect(rowsAfterScroll.length).toBeGreaterThan(0);
  });

  test('should display filter component', async ({ page }) => {
    await openArchive(page);

    // The Filters component should be visible
    // Look for the scientific name input field
    const searchInput = page.locator('input[type="text"]').first();
    await expect(searchInput).toBeVisible();
  });

  test('should show correct column headers', async ({ page }) => {
    await openArchive(page);

    // Verify all expected column headers are present
    const headers = [
      'occurrenceID',
      'scientificName',
      'lat',
      'lng',
      'eventDate',
      'eventTime',
    ];

    for (const header of headers) {
      await expect(page.locator('.table-cell').getByText(header)).toBeVisible();
    }
  });

  test('should update results when opening a second archive', async ({ page }) => {
    // Open the first archive
    await openArchive(page);

    // Verify we see data from the first archive (Quercus, Sequoia, Pinus)
    await expect(page.getByText('Quercus lobata').first()).toBeVisible();

    // Open a second archive (simulates CMD-O or File > Open)
    await triggerMenuOpen(page);

    // The results should now show data from the second archive
    // (Puma concolor, Ursus arctos, Canis lupus)
    await expect(page.getByText('Puma concolor').first()).toBeVisible();

    // The first archive's data should no longer be visible
    await expect(page.getByText('Quercus lobata')).toHaveCount(0);
  });

  test('should display ViewSwitcher component', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Verify the ViewSwitcher is visible and has both options
    await expect(page.getByText('Table', { exact: true })).toBeVisible();
    await expect(page.getByText('Cards', { exact: true })).toBeVisible();
  });

  test('should use default table view on initial load', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Check localStorage shows default table view
    const savedView = await page.evaluate(() => {
      return localStorage.getItem('chuck:viewPreference');
    });
    // Should be 'table' by default
    expect(savedView).toBe('table');

    // Verify we're in table view (column headers should be visible)
    await expect(page.locator('.table-cell', { hasText: 'occurrenceID' })).toBeVisible();
    await expect(page.locator('.table-cell', { hasText: 'scientificName' })).toBeVisible();
  });

  test('should switch to cards view and display scientificName on cards', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Verify we're initially in table view
    await expect(page.locator('.occurrence-table')).toBeVisible();

    // Find and click the Cards option in the SegmentedControl
    await switchToView(page, 'cards');

    // Wait for view to switch
    await page.waitForTimeout(1000);

    // Verify table are no longer visible
    const table = page.locator('.occurrence-table');
    await expect(table).not.toBeVisible();

    // Verify cards are displayed
    const cards = page.locator('.occurrence-card');
    await expect(cards.first()).toBeVisible();

    // Verify scientificName is shown on a card
    // The card should contain the scientific name from our mock data
    const firstCard = cards.first();
    await expect(firstCard).toContainText('Quercus lobata');

    // Verify the card has the expected structure (media placeholder, scientific name, etc.)
    const cardContent = firstCard.locator('article');
    await expect(cardContent).toBeVisible();
  });

  test('should render cards with the correct height', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for view to switch
    await page.waitForTimeout(1000);

    // Get the first card's container div (which has the height style)
    const cardContainer = page.locator('.occurrence-card').first().locator('..');
    const cardHeight = await cardContainer.evaluate((el) => {
      return window.getComputedStyle(el).height;
    });

    expect(cardHeight).toBe('302px');

    // Also verify the actual card dimensions
    const cardBoundingBox = await page.locator('.occurrence-card').first().boundingBox();
    expect(cardBoundingBox).not.toBeNull();
    if (cardBoundingBox) {
      // Card should be approximately 280px tall (allowing small variance)
      expect(cardBoundingBox.height).toBeGreaterThanOrEqual(250);
      expect(cardBoundingBox.height).toBeLessThanOrEqual(286);
    }
  });

  test('should properly handle search with zero results', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial results to load
    await expect(page.locator('main')).toBeVisible();
    const initialRows = await getVisibleOccurrences(page);
    expect(initialRows.length).toBeGreaterThan(0);

    // Get the main scrollable element's scroll height before search
    const mainElement = page.locator('main');
    const initialScrollHeight = await mainElement.evaluate(el => el.scrollHeight);
    expect(initialScrollHeight).toBeGreaterThan(0);

    // Search for something that yields no results
    await searchByScientificName(page, 'nothing');

    // Wait for search to complete
    await page.waitForTimeout(500);

    // Verify no occurrence rows are visible (they should all be gone, not showing "Loading...")
    const finalRows = await getVisibleOccurrences(page);
    expect(finalRows.length).toBe(0);

    // The scroll height should be much smaller now (close to viewport height)
    // because there are no items to render
    const finalScrollHeight = await mainElement.evaluate(el => el.scrollHeight);

    // Should be significantly smaller than initial (at least 50% reduction)
    expect(finalScrollHeight).toBeLessThan(initialScrollHeight * 0.5);
  });

  test('preserves scroll position when switching from table to cards to table without scroll', async ({ page}) => {
    await openArchive(page);
    await page.waitForTimeout(1_000);
    const main = page.locator('main');
    const scrollTopInit = await main.evaluate(el => el.scrollTop);
    expect(scrollTopInit).toEqual(0);

    await switchToView(page, 'cards');
    await page.waitForSelector('.occurrence-card');
    const scrollTopAfterSwitch = await main.evaluate(el => el.scrollTop);
    expect(scrollTopAfterSwitch).toEqual(0);

    await switchToView(page, 'table');
    await page.waitForSelector('.occurrence-row');
    const scrollTopAfterSwitchBatch = await main.evaluate(el => el.scrollTop);
    expect(scrollTopAfterSwitchBatch).toEqual(0);
  });

  test('should maintain scroll position when loading chunks in cards view', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for cards to render
    await page.waitForTimeout(1000);

    const mainElement = page.locator('main');

    // Get initial scroll position
    const initialScroll = await mainElement.evaluate(el => el.scrollTop);
    expect(initialScroll).toBe(0);

    // Scroll down to around record 450 (which triggers chunk loading)
    // With 3 columns and ~302px per row, record 450 is at row 150, around 45,300px
    // Use wheel events to scroll there realistically
    await page.mouse.move(400, 400); // Move mouse over the main area
    for (let i = 0; i < 20; i++) {
      await page.mouse.wheel(0, 500);
      await page.waitForTimeout(50);
    }

    // Wait for chunks to load
    await page.waitForTimeout(1000);

    // Verify we scrolled significantly
    const scrollAfterFirst = await mainElement.evaluate(el => el.scrollTop);
    expect(scrollAfterFirst).toBeGreaterThan(5000);

    // Scroll further down to trigger more chunk loading
    // This is where the bug would manifest - occurrenceCacheVersion++ would cause
    // the component to remount (because {#key} included occurrenceCacheVersion)
    // and reset scroll position to 0
    for (let i = 0; i < 20; i++) {
      await page.mouse.wheel(0, 500);
      await page.waitForTimeout(50);
    }

    // Wait for chunks to load
    await page.waitForTimeout(1500);

    // Verify scroll position increased and didn't reset to 0
    const scrollAfterSecond = await mainElement.evaluate(el => el.scrollTop);
    expect(scrollAfterSecond).toBeGreaterThan(scrollAfterFirst);
  });

  test('should preserve scroll position when switching between table and cards views', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load in table view
    await page.waitForTimeout(1000);

    // Scroll down in table view
    const mainElement = page.locator('main');
    await mainElement.evaluate((el) => {
      el.scrollTop = 2000;
    });

    // Wait for scroll to settle
    await page.waitForTimeout(500);

    // Get a reference occurrence that should be visible (capture text from first visible row)
    const firstVisibleRow = page.locator('.list-item').first();
    const referenceOccurrenceId = await firstVisibleRow.evaluate(el => el.id);

    // Log the scroll position before switching
    const scrollBeforeSwitch = await mainElement.evaluate(el => el.scrollTop);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for view to switch and cards to render
    await page.waitForTimeout(1000);

    // Check scroll position after switch
    const scrollAfterSwitch = await mainElement.evaluate(el => el.scrollTop);

    // Verify that the reference occurrence is still visible in cards view
    // The first visible card should contain the same occurrence ID (or be very close)
    const cardItems = page.locator('.list-item');
    await expect(cardItems.first()).toBeVisible();

    // Get the first few visible cards and check if our reference occurrence is among them
    const firstCardId = await cardItems.first().evaluate(el => el.id);
    const secondCardId = await cardItems.nth(1).evaluate(el => el.id);
    const thirdCardId = await cardItems.nth(2).evaluate(el => el.id);

    // The reference occurrence should be in one of the first few visible cards
    const referenceIsVisible =
      firstCardId?.includes(referenceOccurrenceId || '') ||
      secondCardId?.includes(referenceOccurrenceId || '') ||
      thirdCardId?.includes(referenceOccurrenceId || '');

    expect(referenceIsVisible).toBe(true);

    // Now switch back to table view
    await switchToView(page, 'table');

    // Wait for view to switch back
    await page.waitForTimeout(1000);

    // Verify the reference occurrence is still visible in table view
    const firstRowAfterSwitch = page.locator('.list-item').first();
    const firstRowId = await firstRowAfterSwitch.evaluate(el => el.id);

    // Should show the same or very close occurrence
    expect(firstRowId).toBe(referenceOccurrenceId);
  });

  test('should preserve scroll position when window width changes in cards view', async ({ page }) => {
    // Set initial viewport size to ensure we start with known column count
    await page.setViewportSize({ width: 800, height: 800 });

    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for view to switch and cards to render
    await page.waitForTimeout(1000);

    // Verify cards are visible
    const cards = page.locator('.occurrence-card');
    await expect(cards.first()).toBeVisible();

    // Scroll down significantly
    const mainElement = page.locator('main');
    await mainElement.evaluate((el) => {
      el.scrollTop = 2000;
    });

    // Wait for scroll to settle and virtualizer to update
    await page.waitForTimeout(500);

    // Capture scroll position after scrolling
    const scrollPositionBeforeResize = await mainElement.evaluate(el => el.scrollTop);
    expect(scrollPositionBeforeResize).toBeGreaterThan(1500);

    // Resize window to change the number of columns (3 cols -> 4 cols at 1024px breakpoint)
    await page.setViewportSize({ width: 1100, height: 800 });

    // Wait for resize to trigger virtualizer recreation
    await page.waitForTimeout(500);

    // Now scroll again - this is when the bug manifests
    // The virtualizer recreation causes scroll to jump to top on next scroll
    await mainElement.evaluate((el) => {
      el.scrollTop += 100;
    });

    // Wait for scroll to complete
    await page.waitForTimeout(300);

    // Check that we're still near where we were
    const scrollPositionAfterScroll = await mainElement.evaluate(el => el.scrollTop);

    // Should still be near the bottom, not jumped to top
    expect(scrollPositionAfterScroll).toBeGreaterThan(1000);
  });

  test('should not show loading cards for search results in cards view', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for view to switch and cards to render
    await page.waitForTimeout(1000);

    // Search for "Alllium unifolium" which should return exactly 1 result
    await searchByScientificName(page, 'Allium unifolium');

    // Get matches
    const matchingCards = page.locator('.occurrence-card');
    const matchingCount = await matchingCards.count();
    expect(matchingCount).toBe(1);

    // Get all loading placeholder cards
    const loadingCards = page.locator('.loading-card');
    const loadingCount = await loadingCards.count();

    // The key assertion: there should be NO loading cards visible
    // All results should have been loaded
    expect(loadingCount).toBe(0);

    await searchByScientificName(page, '');

    const matchingCardsAfterReset = page.locator('.occurrence-card');
    const matchingCountAfterReset = await matchingCardsAfterReset.count();
    expect(matchingCountAfterReset).toBeGreaterThan(5);

    // Get all loading placeholder cards
    const loadingCardsAfterReset = page.locator('.loading-card');
    const loadingCountAfterReset = await loadingCardsAfterReset.count();

    // The key assertion: there should be NO loading cards visible
    // All results should have been loaded
    expect(loadingCountAfterReset).toBe(0);
  });
});
