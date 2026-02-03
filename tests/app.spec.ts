import type { Page } from '@playwright/test';
import { expect, test } from '@playwright/test';
import {
  getVisibleOccurrences,
  openArchive,
  searchByScientificName,
  setupMockTauri,
  triggerMenuOpen,
  waitForAppReady,
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
    // Scope to Occurrences tab to avoid conflict with Groups tab ViewSwitcher
    const occTab = page.getByLabel('Occurrences');
    const cardsInput = occTab.locator(`input[type="radio"][value="${view}"]`);
    return cardsInput.click({ force: true });
  }

  test('should display welcome message when no archive is open', async ({
    page,
  }) => {
    // Check for the welcome text
    await expect(
      page.getByText(/Chuck is an application for viewing archives/),
    ).toBeVisible();

    // Check for the Open Archive button
    await expect(
      page.getByRole('button', { name: 'Open Archive' }),
    ).toBeVisible();
  });

  test('should open an archive and display results', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for the main content area to appear
    await expect(page.locator('main')).toBeVisible();
    await page.waitForTimeout(100);

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

  test('should display sort controls in filters sidebar', async ({ page }) => {
    await openArchive(page);

    const filters = page.locator('id=Filters');
    const sortSection = filters.locator('button').filter({ hasText: 'Sort' });
    await expect(sortSection).toBeVisible();
    await sortSection.click();

    // Check for sort by dropdown
    const sortBySelect = page.locator('select#sortBy');
    await expect(sortBySelect).toBeAttached();

    // Check that dropdown has options
    const options = await sortBySelect.locator('option').count();
    expect(options).toBeGreaterThan(1); // At least "None" and one column

    // Select a column to reveal direction dropdown
    await sortBySelect.selectOption({ index: 1 }); // Select first non-None option

    // Check for sort direction dropdown (should appear after selecting a column)
    const sortDirectionSelect = page
      .locator('select')
      .filter({ hasText: 'DESC' });
    await expect(sortDirectionSelect).toBeVisible();
  });

  test('should sort when clicking column headers', async ({ page }) => {
    await openArchive(page);
    await page.waitForTimeout(1000);

    await switchToView(page, 'table');

    // Get the first scientificName before sorting (with default occurrenceID sort, first row is TEST-001)
    // scientificName column is the 2nd column (index 1)
    const firstRow = page.locator('.occurrence-item').first();
    await expect(firstRow).toBeVisible();

    const firstNameBefore = await firstRow
      .locator('.table-cell')
      .nth(2)
      .textContent();
    expect(firstNameBefore).toBe('Quercus lobata');

    // Click on scientificName sort button to sort ASC
    const header = page
      .locator('.table-header-cell')
      .filter({ hasText: 'scientificName' });
    const sortButton = header.locator('.sort-button');
    await sortButton.click();
    await page.waitForTimeout(1000);

    // Should show ascending sort icon in the sort button
    await expect(sortButton.locator('svg')).toBeVisible(); // Arrow icon

    // Get the first scientificName after sorting
    const firstRowAfterSort = page.locator('.occurrence-item').first();
    const firstNameAfter = await firstRowAfterSort
      .locator('.table-cell')
      .nth(2)
      .textContent();

    // The order should have changed - when sorted alphabetically ASC,
    // the first name should be something like "Arbutus" or "Arctostaphylos" (alphabetically before "Quercus")
    expect(firstNameAfter).not.toBe('Quercus lobata');

    // Click again to toggle to descending
    await sortButton.click();
    await page.waitForTimeout(1000);

    // Should still show sort icon (DESC now)
    await expect(sortButton.locator('svg')).toBeVisible();

    // Verify order reversed - should be something alphabetically last
    const firstRowDesc = page.locator('.occurrence-item').first();
    const firstNameDesc = await firstRowDesc
      .locator('.table-cell')
      .nth(2)
      .textContent();
    expect(firstNameDesc).not.toBe(firstNameAfter); // Should be different from ASC

    // Click third time to toggle back to ASC
    await sortButton.click();
    await page.waitForTimeout(1000);

    // Should still show sort icon (back to ASC)
    await expect(sortButton.locator('svg')).toBeVisible();
  });

  test('should default to Core ID sort when opening archive', async ({
    page,
  }) => {
    await openArchive(page);

    // Check that the Core ID column header shows sort icon
    const idHeader = page.locator('.table-header-cell').first();
    await expect(idHeader.locator('svg').first()).toBeVisible();

    // Verify sort dropdown shows the core ID column selected
    const sortBySelect = page.locator('select#sortBy');
    const selectedValue = await sortBySelect.inputValue();
    expect(selectedValue).toBeTruthy(); // Should have a value (the core ID column)
  });

  test('should maintain results when changing sort after scrolling', async ({
    page,
  }) => {
    // Load an archive with 100+ records
    await openArchive(page);

    // Wait for initial results to load
    await page.waitForTimeout(1000);

    // Get the main scrollable element
    const mainElement = page.locator('main');

    // Scroll the page down a bit
    await mainElement.hover();
    await page.mouse.wheel(0, 500);

    // Wait for scroll to settle and chunks to load
    await page.waitForTimeout(1000);

    // Change the Sort By dropdown
    const filters = page.locator('id=Filters');
    const sortSection = filters.locator('button').filter({ hasText: 'Sort' });
    await expect(sortSection).toBeVisible();
    await sortSection.click();
    const sortBySelect = page.locator('select#sortBy');
    await expect(sortBySelect).toBeVisible();
    await sortBySelect.selectOption('eventDate');

    // Wait for sort to apply
    await page.waitForTimeout(1000);

    // Verify the sort actually changed by checking the selected value
    const selectedValue = await sortBySelect.inputValue();
    expect(selectedValue).toBe('eventDate');

    // Assert that there are still rows in the table other than the header
    const rows = await getVisibleOccurrences(page);
    expect(rows.length).toBeGreaterThan(0);
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

  test('should preserve typed text in scientificName field when blurring without selection', async ({
    page,
  }) => {
    await openArchive(page);
    await page.waitForTimeout(1000);

    // Open the Taxonomy accordion
    const taxonomyTrigger = page.locator('text=Taxonomy');
    await taxonomyTrigger.click();

    // Wait for the scientificName field to be visible
    const scientificNameLabel = page.locator(
      '.label .label-text:has-text("scientificName")',
    );
    await scientificNameLabel.waitFor({ state: 'visible', timeout: 5000 });

    // Type in the scientificName field without selecting from autocomplete
    const input = page.locator('input#Combobox-scientificName');
    await input.click();
    await input.fill('Qu');

    // Wait for autocomplete dropdown to appear
    await page.waitForTimeout(500);

    // Verify autocomplete dropdown is visible (get the visible one)
    const dropdown = page.locator('[role="listbox"][data-state="open"]');
    await expect(dropdown).toBeVisible();

    // Click elsewhere (on the label) to blur while autocomplete is open
    await scientificNameLabel.click();

    // Wait for any state updates
    await page.waitForTimeout(500);

    // Verify the text is still in the input field
    await expect(input).toHaveValue('Qu');
  });

  test('should select autocomplete suggestion with ENTER key', async ({
    page,
  }) => {
    await openArchive(page);
    await page.waitForTimeout(1000);

    // Open the Taxonomy accordion
    const taxonomyTrigger = page.locator('text=Taxonomy');
    await taxonomyTrigger.click();

    // Wait for the scientificName field to be visible
    const scientificNameLabel = page.locator(
      '.label .label-text:has-text("scientificName")',
    );
    await scientificNameLabel.waitFor({ state: 'visible', timeout: 5000 });

    // Type in the scientificName field to show autocomplete
    const input = page.locator('input#Combobox-scientificName');
    await input.click();
    await input.fill('Qu');

    // Wait for autocomplete dropdown to appear
    await page.waitForTimeout(500);

    // Verify autocomplete dropdown is visible
    const dropdown = page.locator('[role="listbox"][data-state="open"]');
    await expect(dropdown).toBeVisible();

    // Press down arrow to highlight first suggestion
    await input.press('ArrowDown');
    await page.waitForTimeout(200);

    // Get the highlighted suggestion text
    const highlightedItem = dropdown.locator(
      '[role="option"][data-highlighted]',
    );
    const suggestionText = (await highlightedItem.textContent())?.trim() || '';

    // Press ENTER to select the highlighted suggestion
    // Use page.keyboard to ensure event bubbles through document (required for Chromium)
    await page.keyboard.press('Enter');
    await page.waitForTimeout(300);

    // Verify the input now contains the selected suggestion
    await expect(input).toHaveValue(suggestionText);

    // Verify the dropdown is now closed
    await expect(dropdown).not.toBeVisible();
  });

  test('should clear input text when clicking clear button', async ({
    page,
  }) => {
    await openArchive(page);
    await page.waitForTimeout(1000);

    // Open the Taxonomy accordion
    const taxonomyTrigger = page.locator('text=Taxonomy');
    await taxonomyTrigger.click();

    // Wait for the scientificName field to be visible
    const scientificNameLabel = page.locator(
      '.label .label-text:has-text("scientificName")',
    );
    await scientificNameLabel.waitFor({ state: 'visible', timeout: 5000 });

    // Type in the scientificName field
    const input = page.locator('input#Combobox-scientificName');
    await input.fill('Quercus');
    await page.waitForTimeout(500);

    // Verify input has value
    await expect(input).toHaveValue('Quercus');

    // Click the clear button (X icon)
    const clearButton = page
      .locator('button[aria-label="Clear filter"]')
      .first();
    await clearButton.click();
    await page.waitForTimeout(300);

    // Verify the input is now empty
    await expect(input).toHaveValue('');
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
    // Look for the Filters heading and accordion sections
    const filtersHeading = page.locator('id=Filters');
    await expect(filtersHeading).toBeVisible();

    // Check that accordion sections are present
    const taxonomySection = page.locator('text=Taxonomy');
    await expect(taxonomySection).toBeVisible();
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
      await expect(
        page.locator('.table-header-cell').getByText(header),
      ).toBeVisible();
    }
  });

  test('should update results when opening a second archive', async ({
    page,
  }) => {
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

    // Verify the ViewSwitcher is visible and has both options (scoped to Occurrences tab)
    const occTab = page.getByLabel('Occurrences');
    await expect(occTab.getByText('Table', { exact: true })).toBeVisible();
    await expect(occTab.getByText('Cards', { exact: true })).toBeVisible();
  });

  test('should use default table view on initial load', async ({ page }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Check localStorage shows default table view
    const savedView = await page.evaluate(() => {
      const prefs = JSON.parse(
        localStorage.getItem('chuck:viewPreferences') || '{}',
      );
      return prefs.globalView;
    });
    // Should be 'table' by default
    expect(savedView).toBe('table');

    // Verify we're in table view (column headers should be visible)
    await expect(
      page.locator('.table-header-cell', { hasText: 'occurrenceID' }),
    ).toBeVisible();
    await expect(
      page.locator('.table-header-cell', { hasText: 'scientificName' }),
    ).toBeVisible();
  });

  test('should switch to cards view and display scientificName on cards', async ({
    page,
  }) => {
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

  test('should render cards with the correct height', async ({ page }, testInfo) => {
    test.skip(testInfo.project.name === 'integration-windows', 'Card height calculation differs in Edge');
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for view to switch
    await page.waitForTimeout(1000);

    // Get the first card's container div (which has the height style)
    const cardContainer = page
      .locator('.occurrence-card')
      .first()
      .locator('..');
    const cardHeight = await cardContainer.evaluate((el) => {
      return window.getComputedStyle(el).height;
    });

    expect(cardHeight).toBe('302px');

    // Also verify the actual card dimensions
    const cardBoundingBox = await page
      .locator('.occurrence-card')
      .first()
      .boundingBox();
    expect(cardBoundingBox).not.toBeNull();
    if (cardBoundingBox) {
      // Card should be approximately 280px tall (allowing small variance)
      expect(cardBoundingBox.height).toBeGreaterThanOrEqual(250);
      expect(cardBoundingBox.height).toBeLessThanOrEqual(286);
    }
  });

  test('preserves scroll position when switching from table to cards to table without scroll', async ({
    page,
  }) => {
    await openArchive(page);
    await page.waitForTimeout(1_000);
    const main = page.locator('main');
    const scrollTopInit = await main.evaluate((el) => el.scrollTop);
    expect(scrollTopInit).toEqual(0);

    await switchToView(page, 'cards');
    await page.waitForSelector('.occurrence-card');
    const scrollTopAfterSwitch = await main.evaluate((el) => el.scrollTop);
    expect(scrollTopAfterSwitch).toEqual(0);

    await switchToView(page, 'table');
    await page.waitForSelector('.occurrence-row');
    const scrollTopAfterSwitchBatch = await main.evaluate((el) => el.scrollTop);
    expect(scrollTopAfterSwitchBatch).toEqual(0);
  });

  test('should maintain scroll position when loading chunks in cards view', async ({
    page,
  }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for cards to render
    await page.waitForTimeout(1000);

    const mainElement = page.getByTestId('occurrences-scroll-container');

    // Get initial scroll position
    const initialScroll = await mainElement.evaluate((el) => el.scrollTop);
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
    const scrollAfterFirst = await mainElement.evaluate((el) => el.scrollTop);
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
    const scrollAfterSecond = await mainElement.evaluate((el) => el.scrollTop);
    expect(scrollAfterSecond).toBeGreaterThan(scrollAfterFirst);
  });

  test('should preserve scroll position when switching between table and cards views', async ({
    page,
  }) => {
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
    const firstVisibleRow = page.locator('.occurrence-list-item').first();
    const referenceOccurrenceId = await firstVisibleRow.evaluate((el) => el.id);

    // Switch to cards view
    await switchToView(page, 'cards');

    // Wait for view to switch and cards to render
    await page.waitForTimeout(1000);

    // Verify that the reference occurrence is still visible in cards view
    // The first visible card should contain the same occurrence ID (or be very close)
    const cardItems = page.locator('.occurrence-list-item');
    await expect(cardItems.first()).toBeVisible();

    // Get the first few visible cards and check if our reference occurrence is among them
    const firstCardId = await cardItems.first().evaluate((el) => el.id);
    const secondCardId = await cardItems.nth(1).evaluate((el) => el.id);
    const thirdCardId = await cardItems.nth(2).evaluate((el) => el.id);

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
    const firstRowAfterSwitch = page.locator('.occurrence-list-item').first();
    const firstRowId = await firstRowAfterSwitch.evaluate((el) => el.id);

    // Should show the same or very close occurrence
    expect(firstRowId).toBe(referenceOccurrenceId);
  });

  test('should preserve scroll position when window width changes in cards view', async ({
    page,
  }) => {
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
    const mainElement = page.getByTestId('occurrences-scroll-container');
    await mainElement.evaluate((el) => {
      el.scrollTop = 2000;
    });

    // Wait for scroll to settle and virtualizer to update
    await page.waitForTimeout(500);

    // Capture scroll position after scrolling
    const scrollPositionBeforeResize = await mainElement.evaluate(
      (el) => el.scrollTop,
    );
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
    const scrollPositionAfterScroll = await mainElement.evaluate(
      (el) => el.scrollTop,
    );

    // Should still be near the bottom, not jumped to top
    expect(scrollPositionAfterScroll).toBeGreaterThan(1000);
  });

  test('should not show loading cards for search results in cards view', async ({
    page,
  }) => {
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

  test('should show correct row count after clearing search filter in table view', async ({
    page,
  }) => {
    // Open the archive
    await openArchive(page);

    // Wait for initial load
    await page.waitForTimeout(1000);

    // Record initial row count
    const initialRows = page.locator('main .occurrence-item');
    const initialCount = await initialRows.count();
    expect(initialCount).toBeGreaterThan(10);

    // Search for something (even if mocks don't filter properly, we can still test the clear behavior)
    await searchByScientificName(page, 'Allium unifolium');
    await page.waitForTimeout(1000);

    // Record search row count
    const searchRows = page.locator('main .occurrence-item');
    const searchCount = await searchRows.count();
    expect(searchCount).toEqual(1);

    // Check that there are no "Loading..." rows after search
    const loadingRowsAfterSearch = page.locator(
      'main .occurrence-item:has-text("Loading...")',
    );
    const loadingCountAfterSearch = await loadingRowsAfterSearch.count();
    expect(loadingCountAfterSearch).toBe(0);

    // Clear the search
    await searchByScientificName(page, '');
    await page.waitForTimeout(1000);

    // After clearing, we should be back to initial conditions
    const afterClearRows = page.locator('main .occurrence-item');
    const afterClearCount = await afterClearRows.count();
    expect(afterClearCount).toEqual(initialCount);
  });

  test('should keep header row sticky when scrolling in table view', async ({
    page,
  }) => {
    await openArchive(page);
    await page.waitForTimeout(1000);

    // Get the header row element
    const headerRow = page.locator('.occurrence-table > div').first();
    await expect(headerRow).toBeVisible();

    // Check that the header has sticky positioning
    const position = await headerRow.evaluate((el) => {
      return window.getComputedStyle(el).position;
    });
    expect(position).toBe('sticky');

    // Check that top is set to 0
    const top = await headerRow.evaluate((el) => {
      return window.getComputedStyle(el).top;
    });
    expect(top).toBe('0px');

    // Get initial header position
    const headerBefore = await headerRow.boundingBox();
    expect(headerBefore).not.toBeNull();

    // Scroll down in table view
    const mainElement = page.locator('main');
    await mainElement.evaluate((el) => {
      el.scrollTop = 2000;
    });

    // Wait for scroll to settle
    await page.waitForTimeout(500);

    // Get header position after scroll
    const headerAfter = await headerRow.boundingBox();
    expect(headerAfter).not.toBeNull();

    // Header should still be visible and at the same Y position (sticky)
    if (headerBefore && headerAfter) {
      expect(headerAfter.y).toBe(headerBefore.y);
    }

    // Verify header is still visible
    await expect(headerRow).toBeVisible();
  });
});
