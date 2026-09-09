import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { defaults, idle, type Snapshot } from './types';
export const desktop = isTauri();
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (desktop) return invoke<T>(command, args);
  if (command === 'get_snapshot') return { settings: defaults, status: idle, history: [], hasKey: false } as T & Snapshot;
  if (command === 'list_microphones') return [] as T;
  throw new Error('Open the Bol Windows app to use this feature. This browser is an interface preview.');
}
export async function on<T>(name: string, callback: (value: T) => void) { if (!desktop) return () => {}; return listen<T>(name, event => callback(event.payload)); }
