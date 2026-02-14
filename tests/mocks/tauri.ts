/**
 * Mock implementation of Tauri APIs for Playwright testing.
 * This allows testing the frontend without running the Rust backend.
 *
 * IMPORTANT: This file only exports getInjectionScript().
 * When adding new Tauri commands, add them to the switch statement
 * inside getInjectionScript() only.
 */

import type { ArchiveInfo, SearchResult } from '../../src/lib/types/archive';

/**
 * Script to inject into the page that intercepts Tauri module imports.
 * This must be injected before the app loads.
 */
export function getInjectionScript(
  mockArchive: ArchiveInfo,
  mockSearchResults: SearchResult,
  mockArchive2?: ArchiveInfo,
  mockSearchResults2?: SearchResult,
  customEml?: string,
): string {
  // Serialize the mock data
  const mockArchiveJSON = JSON.stringify(mockArchive);
  const mockSearchResultsJSON = JSON.stringify(mockSearchResults);
  const mockArchive2JSON = mockArchive2 ? JSON.stringify(mockArchive2) : 'null';
  const mockSearchResults2JSON = mockSearchResults2
    ? JSON.stringify(mockSearchResults2)
    : 'null';
  const customEmlJSON = customEml ? JSON.stringify(customEml) : 'null';

  return `
    (function() {
      console.log('[Mock Tauri] Injecting Tauri API mocks');

      const mockArchive = ${mockArchiveJSON};
      const mockSearchResults = ${mockSearchResultsJSON};
      const mockArchive2 = ${mockArchive2JSON};
      const mockSearchResults2 = ${mockSearchResults2JSON};
      const customEml = ${customEmlJSON};

      // Restore state from localStorage to persist across page navigations
      let openCount = parseInt(localStorage.getItem('__MOCK_OPEN_COUNT__') || '0', 10);
      let currentArchive = null;
      let currentSearchResults = null;

      // If an archive was opened in a previous page, restore it
      if (openCount === 1) {
        currentArchive = mockArchive;
        currentSearchResults = mockSearchResults;
      } else if (openCount === 2 && mockArchive2) {
        currentArchive = mockArchive2;
        currentSearchResults = mockSearchResults2;
      }

      let eventListeners = new Map();

      // Mock invoke function
      const mockInvoke = async (command, args) => {
        console.log('[Mock Tauri] invoke called:', command, args);

        // Simulate a non-instant response for all commands
        await new Promise(resolve => setTimeout(resolve, 100));

        switch (command) {
          case 'open_archive':
            openCount++;
            localStorage.setItem('__MOCK_OPEN_COUNT__', openCount.toString());
            if (openCount === 1) {
              currentArchive = mockArchive;
              currentSearchResults = mockSearchResults;
            } else if (openCount === 2 && mockArchive2) {
              currentArchive = mockArchive2;
              currentSearchResults = mockSearchResults2;
            }
            return currentArchive;

          case 'current_archive':
            if (!currentArchive) {
              throw new Error('No archive currently open');
            }
            return currentArchive;

          case 'search': {
            const { limit, offset, searchParams } = args;

            if (!currentSearchResults) {
              return { total: 0, results: [] };
            }

            let filteredResults = currentSearchResults.results;

            // Handle flattened filters structure (matching backend's #[serde(flatten)])
            // All filter fields are at the same level as order_by/order
            if (searchParams) {
              // Extract bounding box params
              const nelat = searchParams.nelat ? parseFloat(searchParams.nelat) : null;
              const nelng = searchParams.nelng ? parseFloat(searchParams.nelng) : null;
              const swlat = searchParams.swlat ? parseFloat(searchParams.swlat) : null;
              const swlng = searchParams.swlng ? parseFloat(searchParams.swlng) : null;

              // Apply bounding box filter if all params present
              if (nelat !== null && nelng !== null && swlat !== null && swlng !== null) {
                const beforeCount = filteredResults.length;
                filteredResults = filteredResults.filter(r => {
                  const lat = r.decimalLatitude;
                  const lng = r.decimalLongitude;
                  if (lat === null || lat === undefined || lng === null || lng === undefined) {
                    return false;
                  }
                  return lat >= swlat && lat <= nelat && lng >= swlng && lng <= nelng;
                });
              }

              // Apply other filters
              for (const [columnName, filterValue] of Object.entries(searchParams)) {
                // Skip reserved sorting fields and bbox fields
                if (columnName === 'sort_by' || columnName === 'sort_direction') continue;
                if (columnName === 'nelat' || columnName === 'nelng' || columnName === 'swlat' || columnName === 'swlng') continue;

                if (filterValue && typeof filterValue === 'string') {
                  filteredResults = filteredResults.filter(r => {
                    const value = r[columnName];
                    return value?.toLowerCase().includes(filterValue.toLowerCase());
                  });
                }
              }
            }

            // Apply sorting if specified
            if (searchParams?.sort_by) {
              const sortBy = searchParams.sort_by;
              const direction = searchParams.sort_direction || 'ASC';

              filteredResults = [...filteredResults].sort((a, b) => {
                const aVal = a[sortBy];
                const bVal = b[sortBy];

                // Handle null/undefined
                if (aVal == null && bVal == null) return 0;
                if (aVal == null) return 1;
                if (bVal == null) return -1;

                // Compare values
                let comparison = 0;
                if (typeof aVal === 'string' && typeof bVal === 'string') {
                  comparison = aVal.localeCompare(bVal);
                } else if (typeof aVal === 'number' && typeof bVal === 'number') {
                  comparison = aVal - bVal;
                } else {
                  comparison = String(aVal).localeCompare(String(bVal));
                }

                return direction === 'DESC' ? -comparison : comparison;
              });
            }

            const paginatedResults = filteredResults.slice(offset, offset + limit);

            return {
              total: filteredResults.length,
              results: paginatedResults,
            };
          }

          case 'get_occurrence': {
            const { occurrenceId } = args;

            if (!currentSearchResults) {
              throw new Error('No archive currently open');
            }

            // Use dynamic core ID column from current archive
            const coreIdColumn = currentArchive?.coreIdColumn || 'occurrenceID';
            const occurrence = currentSearchResults.results.find(
              r => r[coreIdColumn] === occurrenceId
            );

            if (!occurrence) {
              throw new Error('Occurrence not found: ' + occurrenceId);
            }

            return occurrence;
          }

          case 'get_autocomplete_suggestions': {
            const { columnName, searchTerm, limit } = args;

            if (!currentSearchResults) {
              return [];
            }

            // Get unique values from the column that match the search term
            const values = new Set();
            for (const result of currentSearchResults.results) {
              const value = result[columnName];
              if (value && typeof value === 'string') {
                if (value.toLowerCase().includes(searchTerm.toLowerCase())) {
                  values.add(value);
                }
              }
            }

            // Return as sorted array, limited
            return Array.from(values).sort().slice(0, limit || 50);
          }

          case 'aggregate_by_field': {
            const { fieldName, searchParams, limit } = args;

            if (!currentSearchResults) {
              return [];
            }

            let filteredResults = currentSearchResults.results;

            // Apply filters (same logic as search command)
            if (searchParams) {
              // Extract bounding box params
              const nelat = searchParams.nelat ? parseFloat(searchParams.nelat) : null;
              const nelng = searchParams.nelng ? parseFloat(searchParams.nelng) : null;
              const swlat = searchParams.swlat ? parseFloat(searchParams.swlat) : null;
              const swlng = searchParams.swlng ? parseFloat(searchParams.swlng) : null;

              // Apply bounding box filter if all params present
              if (nelat !== null && nelng !== null && swlat !== null && swlng !== null) {
                filteredResults = filteredResults.filter(r => {
                  const lat = r.decimalLatitude;
                  const lng = r.decimalLongitude;
                  if (lat === null || lat === undefined || lng === null || lng === undefined) {
                    return false;
                  }
                  return lat >= swlat && lat <= nelat && lng >= swlng && lng <= nelng;
                });
              }

              // Apply other filters
              for (const [columnName, filterValue] of Object.entries(searchParams)) {
                // Skip sorting parameters (old and new names)
                if (columnName === 'order_by' || columnName === 'order' || columnName === 'sort_by' || columnName === 'sort_direction') continue;
                if (columnName === 'nelat' || columnName === 'nelng' || columnName === 'swlat' || columnName === 'swlng') continue;

                if (filterValue && typeof filterValue === 'string') {
                  filteredResults = filteredResults.filter(r => {
                    const value = r[columnName];
                    return value?.toLowerCase().includes(filterValue.toLowerCase());
                  });
                }
              }
            }

            // Aggregate by field
            const counts = new Map();
            const minOccurrenceIds = new Map(); // Track minimum occurrenceID for each group
            for (const result of filteredResults) {
              const value = result[fieldName];
              const key = value == null ? null : value;
              counts.set(key, (counts.get(key) || 0) + 1);

              // Track the minimum occurrenceID for this group
              const currentMin = minOccurrenceIds.get(key);
              const occurrenceID = result.occurrenceID || result.gbifID;
              if (!currentMin || (occurrenceID && occurrenceID < currentMin)) {
                minOccurrenceIds.set(key, occurrenceID);
              }
            }

            // Convert to array and sort by count descending
            const aggregated = Array.from(counts.entries()).map(([value, count]) => {
              const minOccurrenceID = minOccurrenceIds.get(value);
              // Find the occurrence with this ID to get its photo
              const occurrence = filteredResults.find(r =>
                (r.occurrenceID || r.gbifID) === minOccurrenceID
              );
              const photoUrl = occurrence?.multimedia?.[0]?.identifier || null;

              return {
                value,
                count,
                photoUrl
              };
            }).sort((a, b) => b.count - a.count);

            // Apply limit
            return aggregated.slice(0, limit);
          }

          case 'get_archive_metadata': {
            if (!currentArchive) {
              throw new Error('No archive currently open');
            }

            const defaultEmlContent = \`<?xml version="1.0"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1">
  <dataset>
    <title>Test Archive Dataset</title>
    <creator>
      <individualName>
        <givenName>Test</givenName>
        <surName>Creator</surName>
      </individualName>
      <electronicMailAddress>test@example.org</electronicMailAddress>
    </creator>
    <abstract>
      <para>This is a test dataset for integration testing.</para>
    </abstract>
  </dataset>
</eml:eml>\`;

            const mockMetadata = {
              xml_files: [
                {
                  filename: 'meta.xml',
                  content: \`<?xml version="1.0"?>
<archive>
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.gbif.org/terms/1.0/gbifID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
</archive>\`
                },
                {
                  filename: 'eml.xml',
                  content: customEml || defaultEmlContent
                }
              ]
            };

            return mockMetadata;
          }

          default:
            throw new Error('Unknown command: ' + command);
        }
      };

      // Mock dialog.open function
      const mockOpen = async () => '/mock/path/to/test-archive.zip';

      const mockSave = async () => '/mock/path/to/test-archive.zip';

      // Mock window functions
      const mockGetCurrentWindow = () => ({
        setTitle: (title) => {
          console.log('[Mock Tauri] Window title set to:', title);
        },
      });

      // Mock webview functions
      const mockGetCurrentWebview = () => ({
        onDragDropEvent: async () => {
          return () => {};
        },
      });

      // Mock event listener
      const mockListen = async (event, handler) => {
        console.log('[Mock Tauri] Listening for event:', event);
        eventListeners.set(event, handler);
        return () => {
          console.log('[Mock Tauri] Unlistening from event:', event);
          eventListeners.delete(event);
        };
      };

      // Function to trigger menu-open event (for testing)
      const triggerMenuOpen = () => {
        console.log('[Mock Tauri] Triggering menu-open event');
        const handler = eventListeners.get('menu-open');
        if (handler) {
          handler();
        }
      };

      // Intercept dynamic imports
      const originalImport = window.__import || (async (specifier) => {
        throw new Error('Dynamic import not supported');
      });

      // Override module resolution
      window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
      window.__TAURI_INTERNALS__.invoke = mockInvoke;

      // Store mocks globally for module interception
      window.__MOCK_TAURI__ = {
        invoke: mockInvoke,
        showOpenDialog: mockOpen,
        showSaveDialog: mockSave,
        getCurrentWindow: mockGetCurrentWindow,
        getCurrentWebview: mockGetCurrentWebview,
        listen: mockListen,
        triggerMenuOpen: triggerMenuOpen,
      };

      console.log('[Mock Tauri] Mocks injected successfully');
    })();
  `;
}
