/**
 * Wrapper for Tauri APIs that allows for mocking in tests.
 * In test mode, uses mocks from window.__MOCK_TAURI__.
 * In production, uses real Tauri APIs.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import {
  type EventCallback,
  listen as tauriListen,
} from '@tauri-apps/api/event';
import { getCurrentWindow as tauriGetCurrentWindow } from '@tauri-apps/api/window';
import {
  type OpenDialogOptions,
  type SaveDialogOptions,
  open as tauriOpen,
  save as tauriSave,
} from '@tauri-apps/plugin-dialog';

// Interface for mock Tauri object used in tests
interface MockTauri {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  showOpenDialog(
    options?: OpenDialogOptions,
  ): Promise<string | string[] | null>;
  showSaveDialog(
    options?: SaveDialogOptions,
  ): Promise<string | string[] | null>;
  getCurrentWindow(): ReturnType<typeof tauriGetCurrentWindow>;
  listen<T>(event: string, handler: EventCallback<T>): Promise<() => void>;
}

// Check if we're in test mode with mocks available
const hasMocks = typeof window !== 'undefined' && '__MOCK_TAURI__' in window;

function getMockTauri(): MockTauri {
  return (window as unknown as { __MOCK_TAURI__: MockTauri }).__MOCK_TAURI__;
}

export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (hasMocks) {
    return getMockTauri().invoke(command, args);
  }
  return tauriInvoke<T>(command, args);
}

export async function showOpenDialog(
  options?: OpenDialogOptions,
): Promise<string | string[] | null> {
  if (hasMocks) {
    return getMockTauri().showOpenDialog(options);
  }
  return tauriOpen(options);
}

export async function showSaveDialog(
  options?: SaveDialogOptions,
): Promise<string | string[] | null> {
  if (hasMocks) {
    return getMockTauri().showSaveDialog(options);
  }
  return tauriSave(options);
}

export function getCurrentWindow() {
  if (hasMocks) {
    return getMockTauri().getCurrentWindow();
  }
  return tauriGetCurrentWindow();
}

export async function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<() => void> {
  if (hasMocks) {
    return getMockTauri().listen(event, handler);
  }
  return tauriListen(event, handler);
}

// Basemap types matching the Rust structs

export interface Bounds {
  minLon: number;
  minLat: number;
  maxLon: number;
  maxLat: number;
}

export interface BasemapInfo {
  id: string;
  name: string;
  maxZoom: number;
  bounds: Bounds | null;
  downloadDate: string;
  sourceUrl: string;
  fileSize: number;
}

export async function listBasemaps(): Promise<BasemapInfo[]> {
  return invoke<BasemapInfo[]>('list_basemaps');
}

export async function deleteBasemap(id: string): Promise<void> {
  return invoke('delete_basemap', { id });
}

export async function downloadRegionalBasemap(
  bounds: Bounds,
  maxZoom: number,
  name?: string,
): Promise<void> {
  return invoke('download_regional_basemap', {
    bounds,
    maxZoom,
    name,
  });
}

export async function reverseGeocode(
  lat: number,
  lon: number,
  zoom: number,
): Promise<string> {
  return invoke<string>('reverse_geocode', { lat, lon, zoom });
}

export async function estimateRegionalTiles(
  bounds: Bounds,
  maxZoom: number,
): Promise<{ tiles: number }> {
  return invoke<{ tiles: number }>('estimate_regional_tiles', {
    bounds,
    maxZoom,
  });
}

/**
 * Get the base URL for the tiles custom protocol.
 * On Windows, WebView2 requires http://tiles.localhost/ format.
 * On macOS/Linux, use tiles://localhost/ format.
 */
export function getTileUrlBase(): string {
  const isWindows =
    typeof navigator !== 'undefined' &&
    navigator.userAgent.toLowerCase().includes('windows');
  return isWindows ? 'http://tiles.localhost' : 'tiles://localhost';
}

/**
 * Get the base URL for the basemap custom protocol.
 * Same pattern as getTileUrlBase but for the basemap:// scheme.
 */
export function getBasemapUrlBase(): string {
  const isWindows =
    typeof navigator !== 'undefined' &&
    navigator.userAgent.toLowerCase().includes('windows');
  return isWindows ? 'http://basemap.localhost' : 'basemap://localhost';
}
