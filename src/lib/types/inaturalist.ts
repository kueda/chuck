// Type definitions for iNaturalist API responses

export interface Taxon {
  id: number;
  name: string;
  rank: string;
  rank_level: number;
  iconic_taxon_name?: string;
  preferred_common_name?: string;
  default_photo?: {
    square_url: string;
    medium_url: string;
    url: string;
  };
  ancestor_ids?: number[];
  is_active?: boolean;
  observations_count?: number;
}

export interface Place {
  id: number;
  name: string;
  display_name: string;
  place_type: number;
  bbox_area?: number;
  location?: string;
}

export interface User {
  id: number;
  login: string;
  name?: string;
  icon?: string;
  icon_url?: string;
}

export interface SearchResult {
  taxon?: Taxon;
  place?: Place;
  user?: User;
  record?: Place | User | Taxon;
  type?: string;
  matches?: string[];
  score?: number;
}

export interface ApiResponse<T> {
  total_results: number;
  page: number;
  per_page: number;
  results: T[];
}

export type SourceType = 'taxa' | 'places' | 'users';

export type InatItem = Taxon | Place | User;

export interface ComboboxItem {
  value: string;
  label: string;
  item: InatItem;
}
