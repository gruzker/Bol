import { ArrowRight, Copy, FileText } from 'lucide-react';
import type { Settings } from './types';
import './docs.css';

type Destination = 'home' | 'settings' | 'vocabulary' | 'history';
const chapters = [
  ['setup', 'Set up Bol'], ['first-dictation', 'Your first dictation'],
  ['preferences', 'Make it yours'], ['privacy', 'History & privacy'], ['help', 'Troubleshooting'],
] as const;

export function Docs({ settings, navigate, copy }: {
  settings: Settings; navigate: (page: Destination) => void; copy: (text: string) => void;
}) {
  const shortcut = <kbd>{settings.shortcut.replace('Control', 'Ctrl').replaceAll('+', ' + ')}</kbd>;
  return <div className="docs-page">
    <div className="page-heading"><div><p className="eyebrow">THE BOL GUIDE</p><h1>From setup to your first words.</h1><p>Connect your key, test your microphone, and turn speech into text.</p></div><span className="heading-icon"><FileText size={26}/></span></div>
    <nav className="docs-contents" aria-label="Guide chapters">{chapters.map(([id, label], i) =>
      <button key={id} onClick={() => document.getElementById(`docs-${id}`)?.scrollIntoView({ block: 'start' })}><span>0{i + 1}</span>{label}</button>
    )}</nav>
    <section className="card docs-section" id="docs-setup" aria-labelledby="docs-setup-title">
      <div className="docs-section-title"><span>01</span><h2 id="docs-setup-title">Set up Bol</h2></div>
      <p className="docs-intro">Connect your OpenRouter API key, check your microphone, and start dictating. You need internet access and available OpenRouter credits.</p>
      <ol className="docs-steps">
        <li><h3>Enter your API key</h3><p>Open <b>Settings → API key & microphone</b>. Paste your key into <b>OpenRouter API key</b>, then select <b>Check & save key</b>. The check does not transcribe audio. You still need OpenRouter credits to dictate.</p><p>To get a key, open the address below in your browser. Sign in to OpenRouter or create an account, then create an API key. Keep your key private.</p><div className="docs-address"><code>openrouter.ai/settings/keys</code><button className="text-button" onClick={() => copy('https://openrouter.ai/settings/keys')}><Copy size={14}/>Copy address</button></div></li>
        <li><h3>Check your microphone</h3><p>In the same Settings section, choose your microphone and select <b>Test microphone</b>. Speak a few words. The level meter should move as you talk. Select <b>Stop test</b> to finish. Test audio stays on your PC.</p></li>
      </ol>
      <button className="primary" onClick={() => navigate('settings')}>Open Settings <ArrowRight size={16}/></button>
    </section>
    <section className="card docs-section" id="docs-first-dictation" aria-labelledby="docs-dictation-title">
      <div className="docs-section-title"><span>02</span><h2 id="docs-dictation-title">Your first dictation</h2></div>
      <div className="docs-current"><span>Your current shortcut</span>{shortcut}<b>{settings.activation === 'hold' ? 'Hold to talk' : 'Press to start and stop'}</b></div>
      <ol className="docs-steps">
        <li><h3>Choose where your words should go</h3><p>Open an app such as Notepad and click where you want the text to appear. Stay in that app and leave the cursor there until your text is ready.</p></li>
        <li><h3>Speak, then finish recording</h3><p>{settings.activation === 'hold' ? <>Hold {shortcut} while you speak, then release it to finish.</> : <>Press {shortcut} to start recording. Speak, then press it again to finish.</>} The small icon moves as you speak, then spins while MAI turns your recording into text. Press <kbd>Esc</kbd> to cancel. Each recording can last up to five minutes.</p></li>
        <li><h3>Review your text</h3><p>{settings.autoInsert ? 'Bol tries to insert the text at your cursor. If the text does not appear, open Overview and select Copy on your latest dictation. Paste it into your app.' : 'Automatic insertion is off in your settings. Open Overview, select Copy on your latest dictation, then paste it into your app. To insert text automatically, enable Insert into the active app in Settings.'} Check for any partially inserted text before pasting. Review your words before sending them; Bol does not send messages for you.</p></li>
      </ol>
      <div className="docs-note"><h3>Try a dictation inside Bol</h3><p>In <b>Overview</b>, select <b>Start dictation</b>, speak, then select <b>Stop recording</b>. Select <b>Copy</b> when your text is ready. To dictate into another app, use your keyboard shortcut.</p><button className="text-button" onClick={() => navigate('home')}>Go to Overview <ArrowRight size={14}/></button></div>
    </section>
    <section className="card docs-section" id="docs-preferences" aria-labelledby="docs-preferences-title">
      <div className="docs-section-title"><span>03</span><h2 id="docs-preferences-title">Make it yours</h2></div>
      <dl className="docs-definitions">
        <div><dt>Language & writing style</dt><dd>In <b>Settings → Dictation language</b>, choose <b>Auto-detect</b> when switching languages, including English and Hindi. Select a language to give MAI a specific hint. The list includes all 60 languages supported by MAI Transcribe 2. Bol keeps the writing system returned by MAI. <b>Clean & readable</b> asks for fewer fillers and natural punctuation. <b>As spoken</b> asks to keep fillers and repeated words.</dd></div>
        <div><dt>Shortcut & recording behavior</dt><dd>In Settings, click <b>Keyboard shortcut</b> and press a combination that includes Ctrl or Alt. Select <b>Save shortcut</b>. Under <b>Recording behavior</b>, choose hold-to-talk or press-to-start-and-stop.</dd></div>
        <div><dt>Icon position & appearance</dt><dd>In Settings, choose where the dictation icon appears. It uses the monitor your cursor is on. Choose Light, Dark, or System for the app appearance; System follows your Windows theme.</dd></div>
        <div><dt>Dictionary</dt><dd>In <b>Vocabulary → Dictionary</b>, add names and terms you use often. These give MAI hints for recognizing unfamiliar words. Check the spelling in your transcript.</dd></div>
        <div><dt>Snippets</dt><dd>In <b>Vocabulary → Snippets</b>, enter a short phrase under <b>When I say</b>. Enter the text it should insert under <b>Write this instead</b>, then select <b>Add snippet</b>. For example, save “my email” with your email address. Say “my email” as a complete recording to insert it. The phrase will not expand inside a longer sentence.</dd></div>
        <div><dt>Keep Bol ready</dt><dd>Close the window to keep Bol ready in the system tray near the Windows clock. Click its icon to reopen it. To exit, select <b>Quit Bol</b> in Settings or the tray menu. Enable <b>Launch at login</b> to start Bol when you sign in to Windows.</dd></div>
      </dl>
      <button className="text-button" onClick={() => navigate('vocabulary')}>Open Vocabulary <ArrowRight size={14}/></button>
    </section>
    <section className="card docs-section" id="docs-privacy" aria-labelledby="docs-privacy-title">
      <div className="docs-section-title"><span>04</span><h2 id="docs-privacy-title">History & privacy</h2></div>
      <dl className="docs-definitions">
        <div><dt>Your dictation history</dt><dd>Enable <b>Keep dictation history</b> in Settings to save your latest 500 dictations on this computer. History is off by default. In <b>History</b>, search, copy, or delete saved entries. Turning history off does not delete them. Use <b>Clear history</b> to delete all saved dictations.</dd></div>
        <div><dt>Your audio</dt><dd>Dictation audio is sent to OpenRouter and its transcription provider. Bol does not save audio files. Provider retention policies still apply. The microphone test stays local.</dd></div>
        <div><dt>Your API key</dt><dd>Your saved key is encrypted for your Windows account. To replace it, paste a new key in Settings and select <b>Check & save key</b>. To remove it from Bol, select <b>Remove saved key</b> below the field. To deactivate the key itself, revoke it in OpenRouter.</dd></div>
        <div><dt>Sharing & updating Bol</dt><dd>Share the Bol setup file; each person adds their own API key. To update, quit Bol from its tray menu, run the new installer, and reopen the app.</dd></div>
      </dl>
      <button className="text-button" onClick={() => navigate('history')}>Open History <ArrowRight size={14}/></button>
    </section>
    <section className="card docs-section" id="docs-help" aria-labelledby="docs-help-title">
      <div className="docs-section-title"><span>05</span><h2 id="docs-help-title">Troubleshooting</h2></div>
      <div className="docs-faq">
        <details><summary>The microphone meter does not move</summary><p>Check that the microphone is connected and unmuted. Select the correct input in Bol Settings and test it again. Check Windows microphone privacy settings to allow desktop apps to access the microphone.</p></details>
        <details><summary>The API key check or transcription fails</summary><p>Read the error shown in Bol. Check your internet connection first. In OpenRouter, confirm the key is active, your account has credits, and the key has not reached its spending limit. To replace the key, paste a new one in Settings and select Check & save key. If the error says the provider is temporarily unavailable, try again later.</p></details>
        <details><summary>The shortcut does not start recording</summary><p>Make sure Bol is running in the tray and a key is available. Check your saved shortcut and recording behavior in Settings. If another app uses that combination, choose a different shortcut and select Save shortcut.</p></details>
        <details><summary>My text did not appear in the other app</summary><p>Check that Insert into the active app is enabled. Click a text field before recording and leave the cursor there. If your text is still missing, open Overview and select Copy on your latest dictation. Check for partially inserted text before pasting. Some apps, protected fields, and apps running as administrator can block insertion.</p></details>
        <details><summary>A word or name is wrong</summary><p>Check your selected microphone and try a short recording in a quiet place. Add unfamiliar names to your dictionary to give MAI a hint. Check Dictation language in Settings. Use Auto-detect when switching languages, or select the language you are speaking. Bol keeps the writing system MAI returns. Review the transcript before using it.</p></details>
        <details><summary>I closed the window, but Bol is still running</summary><p>Closing the window hides Bol to the system tray so your shortcut stays available. Right-click its tray icon and select Quit Bol to exit completely.</p></details>
      </div>
    </section>
  </div>;
}
