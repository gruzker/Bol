import languageNames from './languages.json';
export type Theme = 'light' | 'dark' | 'system';
export interface Snippet { trigger: string; text: string }
export interface Settings { theme: Theme; language: keyof typeof languageNames; style: 'clean' | 'verbatim'; shortcut: string; activation: 'hold' | 'toggle'; microphone: string; iconPosition: string; historyEnabled: boolean; autoInsert: boolean; launchAtLogin: boolean; vocabulary: string[]; snippets: Snippet[] }
export interface Status { micTest: boolean; phase: string; message: string; text: string; seconds: number; latencyMs: number; cost: number | null; sessionId: number; }
export interface HistoryItem { id: number; createdAt: number; text: string; language: string; seconds: number; cost: number | null; latencyMs: number }
export interface Snapshot { settings: Settings; status: Status; history: HistoryItem[]; hasKey: boolean }
export const defaults: Settings = { theme: 'system', language: 'auto', style: 'clean', shortcut: 'Control+Shift+Space', activation: 'hold', microphone: '', iconPosition: 'bottom-center', historyEnabled: false, autoInsert: true, launchAtLogin: false, vocabulary: [], snippets: [] };
export const idle: Status = { micTest: false, phase: 'idle', message: 'Ready to dictate', text: '', seconds: 0, latencyMs: 0, cost: null, sessionId: 0 };
export const languages: Record<string, string> = languageNames;

export const iconPositions: Record<string, string> = { 'top-left': 'Top left', 'top-center': 'Top center', 'top-right': 'Top right', 'center-left': 'Left', center: 'Center', 'center-right': 'Right', 'bottom-left': 'Bottom left', 'bottom-center': 'Bottom center', 'bottom-right': 'Bottom right' };
