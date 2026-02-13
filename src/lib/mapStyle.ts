import { layers, namedFlavor } from '@protomaps/basemaps';
import type { StyleSpecification } from 'maplibre-gl';
import { getBasemapUrlBase } from '$lib/tauri-api';

const OSM_ATTRIBUTION =
  '<a target="_blank" href="https://openstreetmap.org/copyright">OpenStreetMap</a>';
const PROTOMAPS_ATTRIBUTION =
  '<a target="_blank" href="https://protomaps.com">Protomaps</a> ' +
  `| ${OSM_ATTRIBUTION}`;

const GLYPHS_URL =
  'https://protomaps.github.io/basemaps-assets/fonts/{fontstack}/{range}.pbf';
const SPRITE_URL =
  'https://protomaps.github.io/basemaps-assets/sprites/v4/light';

/**
 * Build a MapLibre style using the offline Protomaps vector basemap
 * if available, otherwise fall back to online OSM raster tiles.
 */
export function buildMapStyle(hasBasemap: boolean): StyleSpecification {
  if (hasBasemap) {
    return buildVectorStyle();
  }
  return buildRasterStyle();
}

function buildVectorStyle(): StyleSpecification {
  const basemapUrl = getBasemapUrlBase();
  const vectorLayers = (
    layers('basemap', namedFlavor('light'), {
      lang: 'en',
    }) as StyleSpecification['layers']
  ).filter((l) => l.type !== 'background');
  return {
    version: 8,
    glyphs: GLYPHS_URL,
    sprite: SPRITE_URL,
    sources: {
      osm: {
        type: 'raster',
        tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
        tileSize: 256,
        attribution: OSM_ATTRIBUTION,
      },
      basemap: {
        type: 'vector',
        tiles: [`${basemapUrl}/{z}/{x}/{y}`],
        maxzoom: 15,
        attribution: PROTOMAPS_ATTRIBUTION,
      },
    },
    layers: [
      {
        id: 'osm-fallback',
        type: 'raster',
        source: 'osm',
        minzoom: 0,
        maxzoom: 19,
      },
      ...vectorLayers,
    ],
  };
}

function buildRasterStyle(): StyleSpecification {
  return {
    version: 8,
    sources: {
      osm: {
        type: 'raster',
        tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
        tileSize: 256,
        attribution: OSM_ATTRIBUTION,
      },
    },
    layers: [
      {
        id: 'osm',
        type: 'raster',
        source: 'osm',
        minzoom: 0,
        maxzoom: 19,
      },
    ],
  };
}
