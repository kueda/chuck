import { beforeEach, describe, expect, it } from 'vitest';
import {
  getColumnPreferences,
  getViewType,
  saveColumnPreferences,
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
