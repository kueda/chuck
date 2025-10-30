/**
 * Wrapper for Tauri APIs that allows for mocking in tests.
 * In test mode, uses mocks from window.__MOCK_TAURI__.
 * In production, uses real Tauri APIs.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { getCurrentWindow as tauriGetCurrentWindow } from '@tauri-apps/api/window';
import { listen as tauriListen, type EventCallback } from '@tauri-apps/api/event';
import { open as tauriOpen, save as tauriSave } from '@tauri-apps/plugin-dialog';

// Check if we're in test mode with mocks available
const hasMocks = typeof window !== 'undefined' && '__MOCK_TAURI__' in window;

export async function invoke<T>(command: string, args?: any): Promise<T> {
  if (hasMocks) {
    return (window as any).__MOCK_TAURI__.invoke(command, args);
  }
  return tauriInvoke<T>(command, args);
}

export async function showOpenDialog(options?: any): Promise<string | string[] | null> {
  if (hasMocks) {
    return (window as any).__MOCK_TAURI__.showOpenDialog(options);
  }
  return tauriOpen(options);
}

export async function showSaveDialog(options?: any): Promise<string | string[] | null> {
  if (hasMocks) {
    return (window as any).__MOCK_TAURI__.showSaveDialog(options);
  }
  return tauriSave(options);
}

export function getCurrentWindow() {
  if (hasMocks) {
    return (window as any).__MOCK_TAURI__.getCurrentWindow();
  }
  return tauriGetCurrentWindow();
}

export async function listen<T>(
  event: string,
  handler: EventCallback<T>
): Promise<() => void> {
  if (hasMocks) {
    return (window as any).__MOCK_TAURI__.listen(event, handler);
  }
  return tauriListen(event, handler);
}
