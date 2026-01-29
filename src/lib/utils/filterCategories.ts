// Categorizes Darwin Core column names into logical groups for filter organization

import {
  Archive,
  Calendar,
  Globe,
  Leaf,
  SlidersHorizontal,
  User,
} from 'lucide-svelte';
import type { ComponentType } from 'svelte';

// SearchParams matches the backend's flattened structure (using #[serde(flatten)])
// All Darwin Core fields are at the same level as sort_by/sort_direction
export interface SearchParams {
  // Taxonomy
  scientificName?: string;
  genus?: string;
  family?: string;
  order?: string;
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

  // Date / Time
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
  sort_by?: string;
  sort_direction?: 'ASC' | 'DESC';
}

export interface FilterCategory {
  name: string;
  columns: (keyof SearchParams)[];
  icon?: ComponentType;
}

const TAXONOMY_COLUMNS: (keyof SearchParams)[] = [
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

const DATE_TIME_COLUMNS: (keyof SearchParams)[] = [
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

const PERSON_COLUMNS: (keyof SearchParams)[] = ['recordedBy', 'identifiedBy'];

// Synthetic filter fields that don't exist as database columns
const SYNTHETIC_FIELDS: (keyof SearchParams)[] = [
  'nelat',
  'nelng',
  'swlat',
  'swlng',
];

export function categorizeColumns(
  availableColumns: (keyof SearchParams)[],
): FilterCategory[] {
  const categories: FilterCategory[] = [];
  const categorized = new Set<string>();

  // Helper to add category if it has columns
  const addCategory = (
    name: string,
    columnList: (keyof SearchParams)[],
    icon?: ComponentType,
  ) => {
    const matchingColumns = columnList.filter((col) =>
      availableColumns.includes(col),
    );
    if (matchingColumns.length > 0) {
      categories.push({ name, columns: matchingColumns, icon });
      matchingColumns.forEach((col) => {
        categorized.add(col);
      });
    }
  };

  addCategory('Taxonomy', TAXONOMY_COLUMNS, Leaf);

  // Geography gets special handling to always include synthetic bbox fields
  const geographyColumns = GEOGRAPHY_COLUMNS.filter(
    (col) => availableColumns.includes(col) || SYNTHETIC_FIELDS.includes(col),
  );
  if (geographyColumns.length > 0) {
    categories.push({
      name: 'Geography',
      columns: geographyColumns,
      icon: Globe,
    });
    geographyColumns.forEach((col) => {
      categorized.add(col);
    });
  }

  addCategory('Date / Time', DATE_TIME_COLUMNS, Calendar);
  addCategory('Catalog', CATALOG_COLUMNS, Archive);
  addCategory('Person', PERSON_COLUMNS, User);

  // Add "Other" category for uncategorized columns
  const otherColumns = availableColumns.filter((col) => !categorized.has(col));
  if (otherColumns.length > 0) {
    categories.push({
      name: 'Other',
      columns: otherColumns,
      icon: SlidersHorizontal,
    });
  }

  return categories;
}
