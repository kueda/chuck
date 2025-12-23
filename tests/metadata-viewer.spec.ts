import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { setupMockTauri, waitForAppReady, openArchive } from './helpers/setup';
import { parseEML } from '../src/lib/utils/xmlParser';
import { JSDOM } from 'jsdom';

global.DOMParser = new JSDOM().window.DOMParser;

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

test.describe('Metadata Viewer', () => {
  test('shows metadata window with parsed content', async ({ page }) => {
    // Set up mocks and open archive
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await expect(page.locator('main')).toBeVisible();

    // Navigate to metadata route (simulating new window)
    await page.goto('/metadata');

    // Should load metadata
    await expect(page.getByRole('heading', { name: 'Archive Metadata' })).toBeVisible();

    // Should show tabs
    await expect(page.getByRole('tab', { name: 'meta.xml' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'eml.xml' })).toBeVisible();

    // meta.xml tab is first and should be active by default
    // Should show parsed meta content
    await expect(page.getByText('Dataset Title')).toBeVisible();
  });

  test('switches between meta.xml and eml.xml tabs', async ({ page }) => {
    // Set up mocks and open archive
    await setupMockTauri(page);
    await page.goto('/');
    await waitForAppReady(page);
    await openArchive(page);
    await expect(page.locator('main')).toBeVisible();

    // Navigate to metadata route
    await page.goto('/metadata');
    await expect(page.getByRole('heading', { name: 'Archive Metadata' })).toBeVisible();

    // EML tab should be first and active by default
    await expect(page.getByText('Dataset Title')).toBeVisible();

    // Click metafile tab
    const metaTab = page.getByRole('tab', { name: 'meta.xml' });
    await expect(metaTab).toBeVisible();
    await metaTab.click();

    // Should show EML content
    await expect(page.getByText('Core File')).toBeVisible();

    // Click back to meta.xml
    const emlTab = page.getByRole('tab', { name: 'eml.xml' });
    await emlTab.click();

    // Should show meta content
    await expect(page.getByText('Dataset Title')).toBeVisible();
  });

  test('handles missing archive gracefully', async ({ page }) => {
    // Navigate to metadata without opening an archive
    await setupMockTauri(page);
    await page.goto('/metadata');

    // Should show error
    await expect(page.getByText('Error loading metadata')).toBeVisible();
  });

  test.describe('EML', () => {
    const fixturePath = join(__dirname, 'fixtures/eml.xml');
    // Read the EML fixture
    const emlContent = readFileSync(fixturePath, 'utf-8');

    test.beforeEach(async ({ page }) => {
      // Set up mocks with custom EML content
      await setupMockTauri(page, { eml: emlContent });
      await page.goto('/');
      await waitForAppReady(page);
      await openArchive(page);
      await expect(page.locator('main')).toBeVisible();

      // Navigate to metadata route
      await page.goto('/metadata');
      await expect(page.getByRole('heading', { name: 'Archive Metadata' })).toBeVisible();

      // Click eml.xml tab
      const emlTab = page.getByRole('tab', { name: /eml\.xml/i });
      await emlTab.click();
    });

    test('displays metadataProvider', async ({ page }) => {
      const eml = parseEML(emlContent);
      const section = page.locator('section').filter({
        has: page.getByRole('heading', { name: 'Metadata Provider' })
      });
      await expect(section.getByText(eml!.metadataProviders![0].name)).toBeVisible();
      await expect(section.getByText(eml!.metadataProviders![0]!.email!)).toBeVisible();
      await expect(section.getByText(eml!.metadataProviders![0]!.onlineUrl![0])).toBeVisible();
      await expect(section.getByText(eml!.metadataProviders![0]!.phone![0])).toBeVisible();
      await expect(section.getByText(eml!.metadataProviders![0]!.organizationName!)).toBeVisible();
      await expect(section.getByText(eml!.metadataProviders![0]!.positionName!)).toBeVisible();
    });

    test('displays language', async ({ page }) => {
      const eml = parseEML(emlContent);
      await expect(page.getByText(eml!.language!)).toBeVisible();
    });

    test('displays contact', async ({ page }) => {
      const eml = parseEML(emlContent);
      const section = page.locator('section').filter({
        has: page.getByRole('heading', { name: 'Contact' })
      });
      await expect(section.getByText(eml!.contact![0].name)).toBeVisible();
      await expect(section.getByText(eml!.contact![0]!.email!)).toBeVisible();
    });

    test('displays additionalMetadata', async ({ page }) => {
      const eml = parseEML(emlContent);
      const section = page.locator('section').filter({
        has: page.getByRole('heading', { name: 'Additional Metadata' })
      });

      // Should display citation
      await expect(section.getByText('Example University Museum of Natural History, Natural History Collection')).toBeVisible();

      // Should display living time period
      await expect(section.getByText('1900 to present')).toBeVisible();

      // Should display hierarchy level
      await expect(section.getByText('dataset')).toBeVisible();
    });
  });
});
