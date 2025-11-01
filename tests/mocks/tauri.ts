/**
 * Mock implementation of Tauri APIs for Playwright testing.
 * This allows testing the frontend without running the Rust backend.
 */

import type {
  ArchiveInfo,
  SearchResult,
  Occurrence,
} from '../../src/lib/types/archive';

interface MockState {
  currentArchive: ArchiveInfo | null;
  searchResults: Map<string, SearchResult>;
}

/**
 * Creates a stateful mock of Tauri's invoke command system.
 * Maintains state like which archive is currently open.
 */
export function createMockInvoke(
  mockArchive: ArchiveInfo,
  mockSearchResults: SearchResult
) {
  const state: MockState = {
    currentArchive: null,
    searchResults: new Map(),
  };

  return async function invoke<T>(command: string, args?: any): Promise<T> {
    console.log('[Mock Tauri] invoke called:', command, args);

    switch (command) {
      case 'open_archive':
        state.currentArchive = mockArchive;
        return mockArchive as T;

      case 'current_archive':
        if (!state.currentArchive) {
          throw new Error('No archive currently open');
        }
        return state.currentArchive as T;

      case 'search': {
        const { limit, offset, searchParams, fields } = args;

        // Generate cache key from search params
        const cacheKey = JSON.stringify({ searchParams, fields });

        // Filter results based on search params
        let filteredResults = mockSearchResults.results;

        if (searchParams?.scientific_name) {
          filteredResults = filteredResults.filter((r: Occurrence) =>
            r.scientificName?.toLowerCase()
              .includes(searchParams.scientific_name.toLowerCase())
          );
        }

        // Apply pagination
        const paginatedResults = filteredResults.slice(offset, offset + limit);

        const result: SearchResult = {
          total: filteredResults.length,
          results: paginatedResults,
        };

        return result as T;
      }

      default:
        throw new Error(`Unknown command: ${command}`);
    }
  };
}

/**
 * Returns the complete mock object to inject into the browser window.
 * This replaces all Tauri APIs with mock implementations.
 */
export function getMockTauriAPIs(
  mockArchive: ArchiveInfo,
  mockSearchResults: SearchResult
) {
  const mockInvoke = createMockInvoke(mockArchive, mockSearchResults);

  return {
    // Mock @tauri-apps/api/core
    __TAURI_INVOKE__: mockInvoke,

    // Mock @tauri-apps/plugin-dialog
    __TAURI_PLUGIN_DIALOG__: {
      showOpenDialog: async () => '/mock/path/to/test-archive.zip',
      showSaveDialog: async () => '/mock/path/to/test-archive.zip',
    },

    // Mock @tauri-apps/api/window
    __TAURI_WINDOW__: {
      getCurrentWindow: () => ({
        setTitle: (title: string) => {
          console.log('[Mock Tauri] Window title set to:', title);
        },
      }),
    },

    // Mock @tauri-apps/api/event
    __TAURI_EVENT__: {
      listen: async (event: string, handler: Function) => {
        console.log('[Mock Tauri] Listening for event:', event);
        // Return unlisten function
        return () => {
          console.log('[Mock Tauri] Unlistening from event:', event);
        };
      },
    },
  };
}

/**
 * Script to inject into the page that intercepts Tauri module imports.
 * This must be injected before the app loads.
 */
export function getInjectionScript(
  mockArchive: ArchiveInfo,
  mockSearchResults: SearchResult,
  mockArchive2?: ArchiveInfo,
  mockSearchResults2?: SearchResult
): string {
  // Serialize the mock data
  const mockArchiveJSON = JSON.stringify(mockArchive);
  const mockSearchResultsJSON = JSON.stringify(mockSearchResults);
  const mockArchive2JSON = mockArchive2 ? JSON.stringify(mockArchive2) : 'null';
  const mockSearchResults2JSON = mockSearchResults2 ?
    JSON.stringify(mockSearchResults2) : 'null';

  return `
    (function() {
      console.log('[Mock Tauri] Injecting Tauri API mocks');

      const mockArchive = ${mockArchiveJSON};
      const mockSearchResults = ${mockSearchResultsJSON};
      const mockArchive2 = ${mockArchive2JSON};
      const mockSearchResults2 = ${mockSearchResults2JSON};

      let currentArchive = null;
      let currentSearchResults = null;
      let openCount = 0;
      let eventListeners = new Map();

      // Mock invoke function
      const mockInvoke = async (command, args) => {
        console.log('[Mock Tauri] invoke called:', command, args);

        switch (command) {
          case 'open_archive':
            openCount++;
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

            if (searchParams?.scientific_name) {
              filteredResults = filteredResults.filter(r =>
                r.scientificName?.toLowerCase()
                  .includes(searchParams.scientific_name.toLowerCase())
              );
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
        listen: mockListen,
        triggerMenuOpen: triggerMenuOpen,
      };

      console.log('[Mock Tauri] Mocks injected successfully');
    })();
  `;
}
