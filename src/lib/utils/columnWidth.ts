// Returns default pixel width for a column based on field name
export function getDefaultColumnWidth(fieldName: string): number {
  // Coordinate fields
  if (fieldName === 'decimalLatitude' || fieldName === 'decimalLongitude') {
    return 96;
  }

  // Date and time fields
  if (
    fieldName === 'eventDate' ||
    fieldName === 'eventTime' ||
    fieldName === 'dateIdentified' ||
    fieldName === 'modified' ||
    fieldName === 'created'
  ) {
    return 128;
  }

  // ID and numeric fields
  if (
    fieldName.toLowerCase().includes('id') ||
    fieldName === 'year' ||
    fieldName === 'month' ||
    fieldName === 'day'
  ) {
    return 128;
  }

  return 192;
}
