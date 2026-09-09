import { Check, CircleAlert, LoaderCircle } from 'lucide-react';

export function DictationIndicator({ phase, level = 0 }: { phase: string; level?: number }) {
  if (!['recording', 'transcribing', 'done', 'error'].includes(phase)) return null;
  const listening = phase === 'recording';
  const label = listening ? 'Listening — Esc to cancel' : phase === 'transcribing' ? 'Transcribing — Esc to cancel' : phase === 'error' ? 'Dictation failed — open Bol for details' : 'Dictation complete';
  const volume = Math.min(1, Math.max(0, Number.isFinite(level) ? level : 0));
  return <div className={`dictation-icon ${phase}`} role="status" aria-label={label} title={label}>
    {listening ? <span className="dictation-bars" aria-hidden="true">
      {[.4, .75, 1, .75, .4].map((height, i) => <span key={i} style={{ height: 4 + height * (4 + volume * 15), animationDelay: `${i * -0.14}s` }} />)}
    </span> : phase === 'transcribing' ? <LoaderCircle className="spin" size={18} aria-hidden="true" /> : phase === 'error' ? <CircleAlert size={18} aria-hidden="true" /> : <Check size={18} aria-hidden="true" />}
  </div>;
}
