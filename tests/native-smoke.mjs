// Run against an explicitly launched DEBUG build with WebView2 CDP on localhost:9223.
// All microphone recordings below are local tests; none are sent to a provider.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9223');
const page = browser.contexts().flatMap(c => c.pages()).find(p => !p.url().includes('overlay'));
if (!page) throw new Error('Bol main webview not found');
const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
const original = await invoke('get_snapshot');
assert.equal(typeof original.settings.language, 'string');
assert.equal(typeof original.settings.historyEnabled, 'boolean');
try {
  const microphones = await invoke('list_microphones');
  assert(Array.isArray(microphones));
  console.log(`Native microphone enumeration passed (${microphones.length} inputs).`);
  await assert.rejects(invoke('save_settings', { settings: { ...original.settings, shortcut: 'Space' } }));
  assert.equal((await invoke('get_snapshot')).settings.shortcut, original.settings.shortcut);
  await invoke('save_settings', { settings: { ...original.settings, vocabulary: ['Ananya', 'OpenRouter'], snippets: [{ trigger: 'my test phrase', text: 'Saved native snippet' }] } });
  const changed = await invoke('get_snapshot');
  assert.deepEqual(changed.settings.vocabulary, ['Ananya', 'OpenRouter']);
  assert.equal(changed.settings.snippets[0].text, 'Saved native snippet');
  await page.reload();
  await page.getByRole('heading', { name: 'A little less typing.' }).waitFor();
  assert.deepEqual((await invoke('get_snapshot')).settings.vocabulary, ['Ananya', 'OpenRouter']);
  console.log('Settings persisted, invalid hotkey rejected without changing preferences.');
  await invoke('save_settings', { settings: { ...original.settings, microphone: '__missing_microphone_for_test__' } });
  await invoke('start_recording', { micTest: true });
  await page.waitForFunction(async () => (await window.__TAURI_INTERNALS__.invoke('get_snapshot')).status.phase === 'error');
  assert.match((await invoke('get_snapshot')).status.message, /Microphone not found/);
  await invoke('save_settings', { settings: original.settings });
  await invoke('start_recording', { micTest: true });
  await invoke('cancel_recording');
  await new Promise(r => setTimeout(r, 500));
  const cancelled = await invoke('get_snapshot');
  assert.equal(cancelled.status.phase, 'idle');
  assert.equal(cancelled.status.text, '');
  assert.match(cancelled.status.message, /Cancelled/);
  assert.equal(cancelled.history.length, original.history.length);
  console.log('Missing microphone and early cancellation handled; no transcript or history created.');
  if (!original.hasKey) {
    await assert.rejects(invoke('start_recording', { micTest: false }), /API key/);
    console.log('Dictation without a key fails before recording.');
  }
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('heading', { name: 'Make Bol work for you' }).waitFor();
  await page.screenshot({ path: 'artifacts/native-settings.png', fullPage: true });
  assert.equal(await page.getByText('Interface preview', { exact: false }).count(), 0);
  console.log('Actual Windows WebView renders without preview mode. Native smoke checks passed.');
} finally {
  await invoke('cancel_recording');
  await invoke('save_settings', { settings: original.settings });
  await browser.close();
}
