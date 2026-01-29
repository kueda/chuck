const STORAGE_KEY = 'chuck:viewPreferences';

const DEFAULT_COLUMNS = [
  'scientificName',
  'decimalLatitude',
  'decimalLongitude',
  'eventDate',
  'eventTime',
];

interface ViewPreferences {
  globalView?: 'table' | 'cards' | 'map';
  archives?: {
    [archiveName: string]: {
      selectedColumns: string[];
      version?: number;
    };
  };
}

function loadPreferences(): ViewPreferences {
  if (typeof window === 'undefined') {
    return {};
  }

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }

    return {};
  } catch (e) {
    console.error('Failed to load view preferences:', e);
    return {};
  }
}

function savePreferences(prefs: ViewPreferences): void {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch (e) {
    console.error('Failed to save view preferences:', e);
  }
}

export function getViewType(): 'table' | 'cards' | 'map' {
  const prefs = loadPreferences();
  return prefs.globalView || 'table';
}

export function saveViewType(view: 'table' | 'cards' | 'map'): void {
  const prefs = loadPreferences();
  prefs.globalView = view;
  savePreferences(prefs);
}

export function getColumnPreferences(
  archiveName: string,
  coreIdColumn: string,
  availableColumns: string[],
): string[] {
  const prefs = loadPreferences();

  if (prefs.archives?.[archiveName]?.selectedColumns) {
    const validColumns = prefs.archives[archiveName].selectedColumns.filter(
      (col) => availableColumns.includes(col),
    );

    if (validColumns.length > 0) {
      return validColumns;
    }
  }

  // Fallback to defaults
  const defaults = [coreIdColumn, ...DEFAULT_COLUMNS].filter((col) =>
    availableColumns.includes(col),
  );

  // Ensure at least one column
  return defaults.length > 0 ? defaults : [availableColumns[0]];
}

export function saveColumnPreferences(
  archiveName: string,
  selectedColumns: string[],
): void {
  const prefs = loadPreferences();

  if (!prefs.archives) {
    prefs.archives = {};
  }

  prefs.archives[archiveName] = {
    selectedColumns,
    version: 1,
  };

  savePreferences(prefs);
}
