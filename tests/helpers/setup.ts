/**
 * Test helper utilities for setting up Playwright tests with Tauri mocks.
 */

import { type Page } from '@playwright/test';
import { getInjectionScript } from '../mocks/tauri';
import {
  mockArchive,
  mockSearchResult,
  mockArchive2,
  mockSearchResult2,
} from '../fixtures/archive-data';

import type { ArchiveInfo, SearchResult } from '../../src/lib/types/archive';

/**
 * Sets up Tauri API mocks before the page loads.
 * This must be called before navigating to the page.
 */
export async function setupMockTauri(
  page: Page,
  archive: ArchiveInfo = mockArchive,
  searchResult: SearchResult = mockSearchResult
) {
  // Add initialization script that runs before any page code
  await page.addInitScript(
    getInjectionScript(archive, searchResult, mockArchive2, mockSearchResult2)
  );
}

/**
 * Waits for the application to be ready.
 * Useful for waiting for initial rendering to complete.
 */
export async function waitForAppReady(page: Page) {
  // Wait for either the "Open Archive" button or the main content
  await page.waitForSelector('button:has-text("Open Archive"), main', {
    timeout: 10000,
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
export async function searchByScientificName(page: Page, scientificName: string) {
  const searchInput = page.locator('input[type="text"]').first();
  await searchInput.fill(scientificName);

  // Wait a bit for debouncing/search to trigger
  await page.waitForTimeout(500);
}

/**
 * Gets the visible occurrence rows in the virtualizer.
 */
export async function getVisibleOccurrences(page: Page) {
  return page.locator('main .flex.items-center.py-2.px-2.border-b').all();
}

/**
 * Triggers the menu-open event to simulate CMD-O or File > Open
 */
export async function triggerMenuOpen(page: Page) {
  await page.evaluate(() => {
    const mockTauri = (window as any).__MOCK_TAURI__;
    if (mockTauri && mockTauri.triggerMenuOpen) {
      mockTauri.triggerMenuOpen();
    }
  });

  // Wait for archive to load
  await page.waitForTimeout(1000);
}
