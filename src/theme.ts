import { useEffect } from 'react';
import type { Theme } from './types';

const system = window.matchMedia('(prefers-color-scheme: dark)');
export function cachedTheme(): Theme {
  try {
    const value = localStorage.getItem('bol-theme');
    if (value === 'dark' || value === 'light') return value;
  } catch { /* The native preference remains authoritative if browser storage is unavailable. */ }
  return 'system';
}
function apply(theme: Theme) {
  document.documentElement.dataset.theme = theme === 'system' ? (system.matches ? 'dark' : 'light') : theme;
}
export const initialTheme = cachedTheme();
if (!new URLSearchParams(location.search).has('overlay')) apply(initialTheme);

export function useTheme(theme: Theme) {
  useEffect(() => {
    const update = () => apply(theme);
    update();
    try { localStorage.setItem('bol-theme', theme); } catch { /* Optional startup cache. */ }
    system.addEventListener('change', update);
    return () => system.removeEventListener('change', update);
  }, [theme]);
}
