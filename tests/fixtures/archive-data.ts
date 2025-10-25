/**
 * Mock data fixtures for testing.
 * Types are imported from the main application code.
 */

import type {
  ArchiveInfo,
  SearchResult,
  Occurrence,
} from '../../src/lib/types/archive';

export const mockArchive: ArchiveInfo = {
  name: 'Test Darwin Core Archive',
  coreCount: 1000,
  coreIdColumn: 'occurrenceID',
};

export const mockOccurrences: Occurrence[] = [
  {
    occurrenceID: 'TEST-001',
    scientificName: 'Quercus lobata',
    decimalLatitude: 37.7749,
    decimalLongitude: -122.4194,
    eventDate: '2024-01-15',
    eventTime: '10:30:00',
    recordedBy: 'Jane Smith',
    basisOfRecord: 'HumanObservation',
  },
  {
    occurrenceID: 'TEST-002',
    scientificName: 'Sequoia sempervirens',
    decimalLatitude: 37.8044,
    decimalLongitude: -122.2712,
    eventDate: '2024-01-16',
    eventTime: '14:20:00',
    recordedBy: 'John Doe',
    basisOfRecord: 'HumanObservation',
  },
  {
    occurrenceID: 'TEST-003',
    scientificName: 'Pinus ponderosa',
    decimalLatitude: 38.5816,
    decimalLongitude: -121.4944,
    eventDate: '2024-01-17',
    eventTime: '09:15:00',
    recordedBy: 'Jane Smith',
    basisOfRecord: 'HumanObservation',
  },
  {
    occurrenceID: 'TEST-004',
    scientificName: 'Quercus agrifolia',
    decimalLatitude: 34.0522,
    decimalLongitude: -118.2437,
    eventDate: '2024-01-18',
    eventTime: '11:45:00',
    recordedBy: 'Bob Johnson',
    basisOfRecord: 'HumanObservation',
  },
  {
    occurrenceID: 'TEST-005',
    scientificName: 'Sequoia sempervirens',
    decimalLatitude: 40.8021,
    decimalLongitude: -124.1637,
    eventDate: '2024-01-19',
    eventTime: '13:30:00',
    recordedBy: 'Alice Williams',
    basisOfRecord: 'HumanObservation',
  },
];

// Generate additional mock occurrences to simulate a larger dataset
function generateMockOccurrences(count: number): Occurrence[] {
  const species = [
    'Quercus lobata',
    'Sequoia sempervirens',
    'Pinus ponderosa',
    'Quercus agrifolia',
    'Arctostaphylos manzanita',
  ];
  const observers = ['Jane Smith', 'John Doe', 'Bob Johnson', 'Alice Williams'];

  const occurrences: Occurrence[] = [...mockOccurrences];

  for (let i = mockOccurrences.length; i < count-1; i++) {
    occurrences.push({
      occurrenceID: `TEST-${String(i + 1).padStart(3, '0')}`,
      scientificName: species[i % species.length],
      decimalLatitude: 34 + Math.random() * 8,
      decimalLongitude: -124 + Math.random() * 6,
      eventDate: `2024-01-${String((i % 28) + 1).padStart(2, '0')}`,
      eventTime: `${String(Math.floor(Math.random() * 24)).padStart(2, '0')}:${String(
        Math.floor(Math.random() * 60)
      ).padStart(2, '0')}:00`,
      recordedBy: observers[i % observers.length],
      basisOfRecord: 'HumanObservation',
    });
  }

  // Add an occurrence of a unique species to test certain filter conditions
  occurrences.push({
    occurrenceID: `TEST-${count.toString().padStart(3, '0')}`,
    scientificName: "Allium unifolium",
    decimalLatitude: 34 + Math.random() * 8,
    decimalLongitude: -124 + Math.random() * 6,
    eventDate: `2024-01-${String((count % 28) + 1).padStart(2, '0')}`,
    eventTime: `${String(Math.floor(Math.random() * 24)).padStart(2, '0')}:${String(
      Math.floor(Math.random() * 60)
    ).padStart(2, '0')}:00`,
    recordedBy: "Eunice Singleton",
    basisOfRecord: 'HumanObservation',
  });

  return occurrences;
}

export const mockSearchResult: SearchResult = {
  total: 1000,
  results: generateMockOccurrences(1000),
};

// Smaller dataset for quick tests
export const mockSearchResultSmall: SearchResult = {
  total: 5,
  results: mockOccurrences,
};

// Second archive with different data for testing archive switching
export const mockArchive2: ArchiveInfo = {
  name: 'Second Test Archive',
  coreCount: 500,
  coreIdColumn: 'occurrenceID',
};

export const mockOccurrences2: Occurrence[] = [
  {
    occurrenceID: 'ARCHIVE2-001',
    scientificName: 'Puma concolor',
    decimalLatitude: 36.7783,
    decimalLongitude: -119.4179,
    eventDate: '2024-02-01',
    eventTime: '08:00:00',
    recordedBy: 'Wildlife Tracker',
    basisOfRecord: 'HumanObservation',
  },
  {
    occurrenceID: 'ARCHIVE2-002',
    scientificName: 'Ursus arctos',
    decimalLatitude: 37.5000,
    decimalLongitude: -119.5000,
    eventDate: '2024-02-02',
    eventTime: '09:30:00',
    recordedBy: 'Wildlife Tracker',
    basisOfRecord: 'HumanObservation',
  },
  {
    occurrenceID: 'ARCHIVE2-003',
    scientificName: 'Canis lupus',
    decimalLatitude: 38.0000,
    decimalLongitude: -120.0000,
    eventDate: '2024-02-03',
    eventTime: '10:15:00',
    recordedBy: 'Wildlife Tracker',
    basisOfRecord: 'HumanObservation',
  },
];

export const mockSearchResult2: SearchResult = {
  total: 3,
  results: mockOccurrences2,
};

// Large-scale archives for performance testing
export const mockArchiveLarge: ArchiveInfo = {
  name: 'Large Test Archive - 1M records',
  coreCount: 1000000,
  coreIdColumn: 'occurrenceID',
};

export const mockSearchResultLarge: SearchResult = {
  total: 1000000,
  results: generateMockOccurrences(100), // Only generate first 100 for initial display
};

export const mockArchiveSmall: ArchiveInfo = {
  name: 'Small Test Archive - 1K records',
  coreCount: 1000,
  coreIdColumn: 'occurrenceID',
};

export const mockSearchResultSmallScale: SearchResult = {
  total: 1000,
  results: generateMockOccurrences(100),
};

// Export types for use in other test files
export type { ArchiveInfo, Occurrence, SearchResult };
