import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { mkdir } from 'node:fs/promises';

test('interface navigation, honest preview, and keyboard-accessible setup', async () => {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1120, height: 780 } });
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.goto('http://127.0.0.1:1420');
    await page.getByRole('heading', { name: 'A little less typing.' }).waitFor();
    assert.equal(await page.getByText('Interface preview', { exact: false }).count(), 1);
    await page.getByRole('button', { name: 'Set up Bol' }).click();
    await page.getByRole('heading', { name: 'Make Bol work for you' }).waitFor();
    assert.equal(await page.getByLabel('OpenRouter API key').getAttribute('type'), 'password');
    assert.equal(await page.getByLabel('Dictation language').inputValue(), 'auto');
    assert.equal(await page.getByLabel('Dictation language').locator('option').count(), 61);
    assert.deepEqual((await page.getByLabel('Dictation language').locator('option').allTextContents()).slice(0,3), ['Auto-detect', 'English', 'Hindi']);
    await page.getByText('MAI detects your language. Use this when switching languages, including English and Hindi.').waitFor();
    assert.equal(await page.getByText(/Roman Hinglish|Gemini/).count(), 0);
    assert.equal(await page.getByRole('switch', { name: 'Keep dictation history' }).getAttribute('aria-checked'), 'false');
    const shortcut = page.getByLabel('Keyboard shortcut');
    await shortcut.focus();
    await page.keyboard.press('Tab');
    assert.equal(await shortcut.evaluate(el => document.activeElement === el), false);
    await shortcut.focus();
    await page.keyboard.press('Shift+Tab');
    assert.equal(await shortcut.evaluate(el => document.activeElement === el), false);
    await shortcut.focus();
    await page.keyboard.press('Control+Alt+B');
    assert.equal(await shortcut.inputValue(), 'Control+Alt+B');
    await page.keyboard.press('Tab');
    assert.equal(await page.getByRole('button', { name: 'Save shortcut' }).evaluate(el => document.activeElement === el), true);
    await page.getByRole('button', { name: 'Test microphone' }).click();
    await page.getByRole('status').filter({ hasText: 'Open the Bol Windows app' }).waitFor();
    await mkdir('artifacts', { recursive: true });
    await page.screenshot({ path: 'artifacts/settings.png', fullPage: true });
    await page.getByRole('button', { name: 'Vocabulary', exact: true }).click();
    await page.getByLabel('New vocabulary word').fill('Ananya');
    await page.getByRole('button', { name: 'Add word', exact: true }).click();
    await page.getByRole('status').filter({ hasText: 'Open the Bol Windows app' }).waitFor();
    assert.equal(await page.getByRole('button', { name: 'Remove Ananya', exact: true }).count(), 0, 'preview must not pretend to save');
    await page.getByRole('button', { name: /Snippets 0/ }).click();
    await page.getByLabel('When I say').fill('my email');
    await page.getByLabel('Write this instead').fill('hello@example.com');
    assert.equal(await page.getByRole('button', { name: 'Add snippet' }).isEnabled(), true);
    await page.getByRole('button', { name: 'History', exact: true }).click();
    await page.getByLabel('Search dictations').fill('missing');
    await page.getByRole('heading', { name: 'No matching dictations' }).waitFor();
    await page.getByRole('button', { name: 'Overview', exact: true }).click();
    await page.getByLabel('Dictation practice area').fill('kal meeting hai — API v2, at 5pm.');
    assert.equal(await page.getByLabel('Dictation practice area').inputValue(), 'kal meeting hai — API v2, at 5pm.');
    await page.screenshot({ path: 'artifacts/overview.png', fullPage: true });
    await page.setViewportSize({ width: 860, height: 640 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true, 'no horizontal overflow at minimum window size');
    await page.screenshot({ path: 'artifacts/compact.png', fullPage: true });
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
});

test('microphone controls identify test and dictation sessions from native status', async () => {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 860, height: 640 } });
    await page.route('**/src/bridge.ts*', route => route.fulfill({
      contentType: 'application/javascript',
      body: `
        import { defaults, idle } from '/src/types.ts';
        export const desktop = true;
        let status = { ...idle, phase: 'recording', micTest: false, sessionId: 1 };
        const listeners = new Map();
        window.setTestStatus = patch => {
          status = { ...status, ...patch };
          listeners.get('dictation-state')?.forEach(callback => callback(status));
        };
        window.testCommands = [];
        export async function call(command, args) {
          window.testCommands.push({ command, args });
          if (command === 'get_snapshot') return { settings: defaults, status, history: [], hasKey: true };
          if (command === 'list_microphones') return [];
          if (command === 'stop_recording') window.setTestStatus({ phase: 'transcribing' });
          if (command === 'start_recording') window.setTestStatus({ phase: 'recording', micTest: args.micTest, sessionId: status.sessionId + 1 });
        }
        export async function on(name, callback) {
          if (!listeners.has(name)) listeners.set(name, new Set());
          const callbacks = listeners.get(name);
          callbacks.add(callback);
          return () => callbacks.delete(callback);
        }
      `,
    }));
    await page.goto('http://127.0.0.1:1420');
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    const controls = page.locator('.mic-test');
    await controls.getByText(/Dictation audio is sent to OpenRouter/).waitFor();
    assert.equal(await controls.getByText('Test audio stays on this PC', { exact: false }).count(), 0);
    await controls.getByRole('button', { name: 'Stop dictation', exact: true }).click();
    await controls.getByRole('button', { name: 'Transcribing…' }).waitFor();
    assert.equal(await controls.getByRole('button').isDisabled(), true);
    assert.match(await controls.textContent(), /Dictation audio is sent to OpenRouter/);
    await page.evaluate(() => window.setTestStatus({ phase: 'done', text: 'Example dictation' }));
    await controls.getByRole('button', { name: 'Test microphone' }).click();
    await controls.getByRole('button', { name: 'Stop test', exact: true }).waitFor();
    assert.match(await controls.textContent(), /Test audio stays on this PC/);
    await page.getByRole('button', { name: 'Overview', exact: true }).click();
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await controls.getByRole('button', { name: 'Stop test', exact: true }).click();
    await controls.getByRole('button', { name: 'Finishing test' }).waitFor();
    assert.match(await controls.textContent(), /Test audio stays on this PC/);
    const commands = await page.evaluate(() => window.testCommands.filter(c => ['start_recording', 'stop_recording'].includes(c.command)));
    assert.deepEqual(commands.map(c => c.command), ['stop_recording', 'start_recording', 'stop_recording']);
    assert.equal(commands[1].args.micTest, true);
  } finally { await browser.close(); }
});

