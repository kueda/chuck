// Returns Tailwind CSS classes for column width based on field name
export function getColumnWidthClass(fieldName: string): string {
  // Coordinate fields
  if (fieldName === 'decimalLatitude' || fieldName === 'decimalLongitude') {
    return 'w-24';
  }

  // Date and time fields
  if (
    fieldName === 'eventDate' ||
    fieldName === 'eventTime' ||
    fieldName === 'dateIdentified' ||
    fieldName === 'modified' ||
    fieldName === 'created'
  ) {
    return 'w-32';
  }

  // ID and numeric fields
  if (
    fieldName.toLowerCase().includes('id') ||
    fieldName === 'year' ||
    fieldName === 'month' ||
    fieldName === 'day'
  ) {
    return 'w-32';
  }

  // Default to fixed width for text fields (names, descriptions, etc)
  return 'w-48';
}
