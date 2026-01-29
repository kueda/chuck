/**
 * Test helper utilities for setting up Playwright tests with Tauri mocks.
 */

import type { Page } from '@playwright/test';
import type { ArchiveInfo, SearchResult } from '../../src/lib/types/archive';
import {
  mockArchive,
  mockArchive2,
  mockSearchResult,
  mockSearchResult2,
} from '../fixtures/archive-data';
import { getInjectionScript } from '../mocks/tauri';

export interface SetupMockTauriOptions {
  archive?: ArchiveInfo;
  searchResult?: SearchResult;
  eml?: string;
}

/**
 * Sets up Tauri API mocks before the page loads.
 * This must be called before navigating to the page.
 */
export async function setupMockTauri(
  page: Page,
  options?: SetupMockTauriOptions,
) {
  const archive = options?.archive ?? mockArchive;
  const searchResult = options?.searchResult ?? mockSearchResult;
  const eml = options?.eml;

  // Add initialization script that runs before any page code
  await page.addInitScript(
    getInjectionScript(
      archive,
      searchResult,
      mockArchive2,
      mockSearchResult2,
      eml,
    ),
  );
}

/**
 * Waits for the application to be ready.
 * Useful for waiting for initial rendering to complete.
 */
export async function waitForAppReady(page: Page) {
  // Wait for either the "Open Archive" button or the main content
  await page.waitForSelector('button:has-text("Open Archive"), main', {
    timeout: 10_000,
  });
}

/**
 * Opens an archive in the test application.
 */
export async function openArchive(page: Page) {
  const openButton = page.getByRole('button', { name: 'Open Archive' });
  await openButton.click();

  // Wait for archive to load and results to appear
  await page.waitForSelector('main', { timeout: 5000 });
}

/**
 * Performs a search with the given scientific name.
 */
export async function searchByScientificName(
  page: Page,
  scientificName: string,
) {
  // Check if Taxonomy accordion is already open by checking if scientificName label is visible
  const scientificNameLabel = page.locator(
    '.label .label-text:has-text("scientificName")',
  );
  const isVisible = await scientificNameLabel.isVisible().catch(() => false);

  // Only click the accordion if it's not already open
  if (!isVisible) {
    const taxonomyTrigger = page.locator('text=Taxonomy');
    await taxonomyTrigger.click();

    // Wait for accordion content to appear
    await scientificNameLabel.waitFor({ state: 'visible', timeout: 5000 });
  }

  // If empty string, clear the filter using the clear button
  if (!scientificName || scientificName.trim() === '') {
    const clearButton = scientificNameLabel
      .locator('..')
      .locator('..')
      .locator('button[aria-label="Clear filter"]');
    const clearButtonCount = await clearButton.count();
    if (clearButtonCount > 0) {
      await clearButton.click();
      await page.waitForTimeout(500);
    }
    return;
  }

  // Find the combobox input within the same filter component
  const searchInput = scientificNameLabel
    .locator('..')
    .locator('..')
    .locator('input[role="combobox"]');
  await searchInput.fill(scientificName);

  // Wait for search to complete
  await page.waitForTimeout(500);
}

/**
 * Gets the visible occurrence rows in the virtualizer (excludes header).
 */
export async function getVisibleOccurrences(page: Page) {
  return page
    .locator('main .occurrence-item')
    .filter({ hasNotText: 'Loading...' })
    .all();
}

/**
 * Triggers the menu-open event to simulate CMD-O or File > Open
 */
export async function triggerMenuOpen(page: Page) {
  await page.evaluate(() => {
    const mockTauri = (window as any).__MOCK_TAURI__;
    if (mockTauri?.triggerMenuOpen) {
      mockTauri.triggerMenuOpen();
    }
  });

  // Wait for archive to load
  await page.waitForTimeout(1000);
}

/**
 * Switches to a different view (table, cards, or map)
 */
export async function switchToView(page: Page, view: string) {
  // Scope to Occurrences tab to avoid conflict with Groups tab ViewSwitcher
  const occTab = page.getByLabel('Occurrences');
  const viewInput = occTab.locator(`input[type="radio"][value="${view}"]`);
  return viewInput.click({ force: true });
}
