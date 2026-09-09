# Contributing to Bol

Bol is a Windows dictation app built with React, TypeScript, Tauri, and Rust.
Read [README.md](README.md) for setup and architecture limitations.

## Development

Install Node.js 22+, Rust stable, Visual Studio C++ Build Tools, and WebView2.
Run `npm ci`, then `npm run tauri -- dev` from the project root.
Use your own OpenRouter key for manual dictation. Never commit credentials,
personal audio, transcripts, or local app data.

## Checks

Run `npm run build`. With `npm run dev` running in another terminal on port
1420, run `npm test` (requires Microsoft Edge). From `src-tauri`, run
`cargo test --lib` and `cargo clippy --all-targets -- -D warnings`.
The ignored live transcription test makes paid requests and is not part of
ordinary validation. Native smoke tests require a separate debug app and may
change local preferences temporarily; use a disposable Windows test account.

## Changes and bug reports

Keep changes focused. Explain the problem, resulting behavior, and checks run.
For bugs, include Bol and Windows versions, reproduction steps, and expected
and actual behavior. Redact keys and private transcript content. Use the
repository issue tracker once it is published; see [SECURITY.md](SECURITY.md)
for sensitive reports.

Contributions must be code you have the right to submit under Bol's MIT
license. Preserve third-party licenses and notices. When dependencies change,
regenerate `THIRD_PARTY_NOTICES.md` with `python scripts/generate-notices.py`
after installing locked dependencies. This requires Python 3.11+, Cargo,
and network access for upstream notices missing from package archives.
