// Categorizes Darwin Core column names into logical groups for filter organization

// SearchParams matches the backend's flattened structure (using #[serde(flatten)])
// All Darwin Core fields are at the same level as order_by/order
// Note: The Darwin Core "order" taxonomy field conflicts with the sort "order" field,
// so it cannot be filtered (backend reserves "order" and "order_by")
export interface SearchParams {
  // Taxonomy
  scientificName?: string;
  genus?: string;
  family?: string;
  // order?: string;  // CONFLICT: reserved for sort direction
  class?: string;
  phylum?: string;
  kingdom?: string;
  taxonRank?: string;
  taxonomicStatus?: string;
  vernacularName?: string;
  specificEpithet?: string;
  infraspecificEpithet?: string;
  taxonID?: string;
  higherClassification?: string;
  subfamily?: string;
  tribe?: string;
  subtribe?: string;
  superfamily?: string;
  subgenus?: string;
  genericName?: string;
  infragenericEpithet?: string;
  species?: string;

  // Geography
  decimalLatitude?: number;
  decimalLongitude?: number;
  coordinateUncertaintyInMeters?: number;
  coordinatePrecision?: number;
  locality?: string;
  stateProvince?: string;
  county?: string;
  municipality?: string;
  country?: string;
  countryCode?: string;
  continent?: string;
  waterBody?: string;
  island?: string;
  islandGroup?: string;
  higherGeography?: string;
  verbatimLocality?: string;

  // Bounding box (northeast/southwest corners)
  nelat?: string;
  nelng?: string;
  swlat?: string;
  swlng?: string;

  // Temporal
  eventDate?: string;
  eventTime?: string;
  year?: string;
  month?: string;
  day?: string;
  dateIdentified?: string;
  modified?: string;
  created?: string;
  verbatimEventDate?: string;

  // Identification
  gbifID?: number;
  recordedBy?: string;
  identifiedBy?: string;
  recordNumber?: string;
  catalogNumber?: string;
  otherCatalogNumbers?: string;
  fieldNumber?: string;
  occurrenceID?: string;
  institutionCode?: string;
  collectionCode?: string;

  // Sorting (reserved field names, not filters)
  order_by?: string;
  order?: 'ASC' | 'DESC';
}

export interface FilterCategory {
  name: string;
  columns: (keyof SearchParams)[];
}

const TAXONOMY_COLUMNS: (keyof SearchParams)[] = [
  'scientificName',
  'kingdom',
  'phylum',
  'class',
  // TODO: figure out a way to support filtering by order that doesn't conflict with sort control
  // 'order',
  'superfamily',
  'family',
  'subfamily',
  'tribe',
  'subtribe',
  'genus',
  'genericName',
  'infragenericEpithet',
  'subgenus',
  'species',
  'specificEpithet',
  'infraspecificEpithet',
  'taxonRank',
  'taxonomicStatus',
  'vernacularName',
  'taxonID',
  'higherClassification',
];

const GEOGRAPHY_COLUMNS: (keyof SearchParams)[] = [
  'decimalLatitude',
  'decimalLongitude',
  'coordinateUncertaintyInMeters',
  'coordinatePrecision',
  'locality',
  'verbatimLocality',
  'continent',
  'country',
  'countryCode',
  'stateProvince',
  'county',
  'municipality',
  'waterBody',
  'island',
  'islandGroup',
  'higherGeography',
  'nelat',
  'nelng',
  'swlat',
  'swlng',
];

const TEMPORAL_COLUMNS: (keyof SearchParams)[] = [
  'eventDate',
  'eventTime',
  'year',
  'month',
  'day',
  'dateIdentified',
  'modified',
  'created',
  'verbatimEventDate',
];

const CATALOG_COLUMNS: (keyof SearchParams)[] = [
  'recordNumber',
  'catalogNumber',
  'otherCatalogNumbers',
  'fieldNumber',
  'occurrenceID',
  'institutionCode',
  'collectionCode',
];

const PERSON_COLUMNS: (keyof SearchParams)[] = [
  'recordedBy',
  'identifiedBy',
]

// Synthetic filter fields that don't exist as database columns
const SYNTHETIC_FIELDS: (keyof SearchParams)[] = ['nelat', 'nelng', 'swlat', 'swlng'];

export function categorizeColumns(availableColumns: (keyof SearchParams)[]): FilterCategory[] {
  const categories: FilterCategory[] = [];
  const categorized = new Set<string>();

  // Helper to add category if it has columns
  const addCategory = (name: string, columnList: (keyof SearchParams)[]) => {
    const matchingColumns = columnList.filter(col => availableColumns.includes(col));
    if (matchingColumns.length > 0) {
      categories.push({ name, columns: matchingColumns });
      matchingColumns.forEach(col => categorized.add(col));
    }
  };

  addCategory('Taxonomy', TAXONOMY_COLUMNS);

  // Geography gets special handling to always include synthetic bbox fields
  const geographyColumns = GEOGRAPHY_COLUMNS.filter(col =>
    availableColumns.includes(col) || SYNTHETIC_FIELDS.includes(col)
  );
  if (geographyColumns.length > 0) {
    categories.push({ name: 'Geography', columns: geographyColumns });
    geographyColumns.forEach(col => categorized.add(col));
  }

  addCategory('Temporal', TEMPORAL_COLUMNS);
  addCategory('Catalog', CATALOG_COLUMNS);
  addCategory('Person', PERSON_COLUMNS);

  // Add "Other" category for uncategorized columns
  const otherColumns = availableColumns.filter(col => !categorized.has(col));
  if (otherColumns.length > 0) {
    categories.push({ name: 'Other', columns: otherColumns });
  }

  return categories;
}
