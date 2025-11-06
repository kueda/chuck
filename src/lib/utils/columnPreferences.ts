const STORAGE_KEY = 'chuck:columnPreferences';

const DEFAULT_COLUMNS = [
  'scientificName',
  'decimalLatitude',
  'decimalLongitude',
  'eventDate',
  'eventTime'
];

interface ColumnPreferences {
  [archiveName: string]: {
    selectedColumns: string[];
    version?: number;
  };
}

export function getColumnPreferences(
  archiveName: string,
  coreIdColumn: string,
  availableColumns: string[]
): string[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    const prefs: ColumnPreferences = stored ? JSON.parse(stored) : {};

    if (prefs[archiveName]?.selectedColumns) {
      const validColumns = prefs[archiveName].selectedColumns.filter(
        col => availableColumns.includes(col)
      );

      if (validColumns.length > 0) {
        return validColumns;
      }
    }
  } catch (e) {
    console.error('Failed to load column preferences:', e);
  }

  // Fallback to defaults
  const defaults = [coreIdColumn, ...DEFAULT_COLUMNS].filter(
    col => availableColumns.includes(col)
  );

  // Ensure at least one column
  return defaults.length > 0 ? defaults : [availableColumns[0]];
}

export function saveColumnPreferences(
  archiveName: string,
  selectedColumns: string[]
): void {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    const prefs: ColumnPreferences = stored ? JSON.parse(stored) : {};

    prefs[archiveName] = {
      selectedColumns,
      version: 1
    };

    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch (e) {
    console.error('Failed to save column preferences:', e);
  }
}
