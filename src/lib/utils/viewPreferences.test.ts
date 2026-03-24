import { beforeEach, describe, expect, it } from 'vitest';
import {
  getColumnPreferences,
  getColumnWidthPreferences,
  getViewType,
  saveColumnPreferences,
  saveColumnWidthPreferences,
  saveViewType,
} from './viewPreferences';

describe('viewPreferences', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe('getViewType', () => {
    it('returns table as default when no preference saved', () => {
      expect(getViewType()).toBe('table');
    });

    it('returns saved view type', () => {
      saveViewType('cards');
      expect(getViewType()).toBe('cards');
    });
  });

  describe('saveViewType', () => {
    it('saves view type to localStorage', () => {
      saveViewType('cards');
      const stored = JSON.parse(
        localStorage.getItem('chuck:viewPreferences') || '{}',
      );
      expect(stored.globalView).toBe('cards');
    });
  });

  describe('getColumnWidthPreferences', () => {
    it('returns empty object when no preference saved', () => {
      expect(getColumnWidthPreferences('test-archive')).toEqual({});
    });

    it('returns saved column widths', () => {
      saveColumnWidthPreferences('test-archive', {
        scientificName: 200,
        eventDate: 100,
      });
      expect(getColumnWidthPreferences('test-archive')).toEqual({
        scientificName: 200,
        eventDate: 100,
      });
    });

    it('returns empty object for unknown archive', () => {
      saveColumnWidthPreferences('other-archive', { foo: 150 });
      expect(getColumnWidthPreferences('test-archive')).toEqual({});
    });

    it('filters out non-numeric widths from corrupt data', () => {
      localStorage.setItem(
        'chuck:viewPreferences',
        JSON.stringify({
          archives: {
            'test-archive': {
              selectedColumns: [],
              columnWidths: { good: 150, bad: 'wide', alsoGood: 80 },
            },
          },
        }),
      );
      expect(getColumnWidthPreferences('test-archive')).toEqual({
        good: 150,
        alsoGood: 80,
      });
    });

    it('filters out non-positive widths from corrupt data', () => {
      localStorage.setItem(
        'chuck:viewPreferences',
        JSON.stringify({
          archives: {
            'test-archive': {
              selectedColumns: [],
              columnWidths: {
                zero: 0,
                negative: -50,
                nan: NaN,
                inf: Infinity,
                valid: 120,
              },
            },
          },
        }),
      );
      expect(getColumnWidthPreferences('test-archive')).toEqual({ valid: 120 });
    });
  });

  describe('saveColumnWidthPreferences', () => {
    it('saves without overwriting selectedColumns', () => {
      saveColumnPreferences('test-archive', ['id', 'name']);
      saveColumnWidthPreferences('test-archive', { id: 80 });
      const result = getColumnPreferences('test-archive', 'id', ['id', 'name']);
      expect(result).toEqual(['id', 'name']);
      expect(getColumnWidthPreferences('test-archive')).toEqual({ id: 80 });
    });
  });

  describe('saveColumnPreferences', () => {
    it('does not overwrite saved column widths', () => {
      saveColumnWidthPreferences('test-archive', { id: 80 });
      saveColumnPreferences('test-archive', ['id', 'name']);
      expect(getColumnWidthPreferences('test-archive')).toEqual({ id: 80 });
    });
  });

  describe('getColumnPreferences', () => {
    it('returns defaults when no preference saved', () => {
      const result = getColumnPreferences('test-archive', 'id', [
        'id',
        'scientificName',
        'eventDate',
      ]);
      expect(result).toContain('id');
      expect(result).toContain('scientificName');
    });

    it('returns saved column preferences', () => {
      saveColumnPreferences('test-archive', ['id', 'foo']);
      const result = getColumnPreferences('test-archive', 'id', [
        'id',
        'foo',
        'bar',
      ]);
      expect(result).toEqual(['id', 'foo']);
    });
  });
});
