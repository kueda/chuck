// Categorizes Darwin Core column names into logical groups for filter organization

export interface FilterCategory {
  name: string;
  columns: string[];
}

const TAXONOMY_COLUMNS = [
  'scientificName',
  'kingdom',
  'phylum',
  'class',
  'order',
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

const GEOGRAPHY_COLUMNS = [
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
];

const TEMPORAL_COLUMNS = [
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

const CATALOG_COLUMNS = [
  'recordNumber',
  'catalogNumber',
  'otherCatalogNumbers',
  'fieldNumber',
  'occurrenceID',
  'institutionCode',
  'collectionCode',
];

const PERSON_COLUMNS = [
  'recordedBy',
  'identifiedBy',
]

export function categorizeColumns(availableColumns: string[]): FilterCategory[] {
  const categories: FilterCategory[] = [];
  const categorized = new Set<string>();

  // Helper to add category if it has columns
  const addCategory = (name: string, columnList: string[]) => {
    const matchingColumns = columnList.filter(col => availableColumns.includes(col));
    if (matchingColumns.length > 0) {
      categories.push({ name, columns: matchingColumns });
      matchingColumns.forEach(col => categorized.add(col));
    }
  };

  addCategory('Taxonomy', TAXONOMY_COLUMNS);
  addCategory('Geography', GEOGRAPHY_COLUMNS);
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
