"""Collect notices for locked Rust and installed npm packages; Python 3.11+."""
import json
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
PREFIXES = ('license', 'licence', 'copying', 'notice', 'copyright', 'unlicense')


def notices(directory, recursive=True):
    paths = directory.rglob('*') if recursive else directory.iterdir()
    return [(str(p.relative_to(directory)), p.read_text(encoding='utf-8', errors='replace'))
            for p in sorted(paths) if p.is_file() and p.name.lower().startswith(PREFIXES)
            and p.suffix.lower() not in ('.rs', '.py', '.json', '.html')]


def upstream_notices(package, directory):
    repo = (package.get('repository') or '').removesuffix('/').removesuffix('.git')
    if not repo.startswith('https://github.com/'):
        raise RuntimeError(f"No packaged notices: {package['name']}")
    vcs = json.loads((directory / '.cargo_vcs_info.json').read_text(encoding='utf-8'))
    commit = vcs['git']['sha1']
    base = repo.replace('https://github.com/', 'https://raw.githubusercontent.com/') + '/' + commit + '/'
    cache_file = ROOT / 'artifacts' / 'license-cache' / f"{repo.split('/')[-1]}-{commit}.json"
    if cache_file.exists():
        return json.loads(cache_file.read_text(encoding='utf-8'))
    print(f"Fetching upstream notices: {package['name']}", flush=True)
    def fetch(name):
        try:
            with urlopen(base + name, timeout=20) as response:
                return (base + name, response.read().decode('utf-8'))
        except HTTPError as error:
            if error.code != 404:
                raise
        return None
    with ThreadPoolExecutor(max_workers=5) as pool:
        result = [value for value in pool.map(fetch, ['LICENSE', 'LICENSE.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE-MIT.txt', 'LICENSE-APACHE.txt', 'LICENSE_MIT', 'LICENSE_APACHE', 'COPYING', 'NOTICE']) if value]
    if not result and package.get('license') == 'MPL-2.0':
        url = 'https://www.mozilla.org/media/MPL/2.0/index.txt'
        with urlopen(url, timeout=20) as response:
            result.append((url, response.read().decode('utf-8')))
    if not result:
        raise RuntimeError(f"Missing upstream notices: {package['name']} at {commit}")
    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(json.dumps(result), encoding='utf-8')
    return result


def main():
    metadata = json.loads(subprocess.check_output([
        'cargo', 'metadata', '--manifest-path', str(ROOT / 'src-tauri/Cargo.toml'),
        '--format-version', '1', '--locked', '--offline',
        '--filter-platform', 'x86_64-pc-windows-msvc'], cwd=ROOT))
    entries = []
    cache = {}
    for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
        if not package['source']:
            continue
        directory = Path(package['manifest_path']).parent
        texts = notices(directory)
        if not texts:
            key = (package.get('repository'), json.loads((directory / '.cargo_vcs_info.json').read_text(encoding='utf-8'))['git']['sha1'])
            if key not in cache:
                cache[key] = upstream_notices(package, directory)
            texts = cache[key]
        entries.append((f"Rust: {package['name']} {package['version']}", package.get('license'),
                        f"https://crates.io/crates/{package['name']}/{package['version']}", texts))
    lock = json.loads((ROOT / 'package-lock.json').read_text(encoding='utf-8'))
    for relative, info in sorted(lock['packages'].items()):
        if not relative:
            continue
        directory = ROOT / relative
        if not directory.exists():
            if info.get('optional'):
                continue
            raise RuntimeError(f'Install dependencies first: {relative}')
        package = json.loads((directory / 'package.json').read_text(encoding='utf-8'))
        texts = notices(directory, recursive=False)
        # Platform binaries are published from the same project/version as their wrapper.
        wrappers = {'@esbuild/': 'esbuild', '@rollup/rollup-': 'rollup', '@tauri-apps/cli-': '@tauri-apps/cli'}
        for prefix, wrapper in wrappers.items():
            if not texts and package['name'].startswith(prefix):
                wrapper_dir = ROOT / 'node_modules' / wrapper
                wrapper_info = json.loads((wrapper_dir / 'package.json').read_text(encoding='utf-8'))
                if wrapper_info['version'] != package['version']:
                    raise RuntimeError(f'Wrapper version mismatch: {relative}')
                texts = [(f'{wrapper}/{name}', text) for name, text in notices(wrapper_dir, recursive=False)]
        if not texts:
            raise RuntimeError(f'Missing npm notices: {relative}')
        entries.append((f"npm: {package['name']} {package['version']}", package.get('license'),
                        f"https://www.npmjs.com/package/{package['name']}/v/{package['version']}", texts))
    output = ['# Third-party notices', '',
              'Bol is MIT-licensed; third-party components retain their own licenses.',
              'This conservative inventory includes Windows Rust dependencies and build tools,',
              'plus installed npm development and runtime packages from the lockfiles.',
              'Not every listed package is shipped in the executable. Original license',
              'expressions and notice texts are preserved, including dual-license options.',
              '', 'MPL-2.0 components are unmodified upstream packages. Their exact source',
              'versions are available through the linked crates.io pages and downloadable',
              'crate archives. Their source remains under MPL-2.0, not Bol\'s MIT license.',
              '', 'OpenRouter, MAI Transcribe 2, Windows, and WebView2 are external services',
              'or platform software governed by their own terms; Bol does not relicense them.',
              '', 'Generated by `python scripts/generate-notices.py`.', '']
    for title, license_id, source, texts in entries:
        output += [f'## {title}', '', f'License expression: `{license_id}`', '', f'Source: {source}', '']
        for name, content in texts:
            output += [f'### {name}', '', '````text', content.strip(), '````', '']
    (ROOT / 'THIRD_PARTY_NOTICES.md').write_text('\n'.join(output), encoding='utf-8')
    print(f'Generated notices for {len(entries)} packages.')


if __name__ == '__main__':
    main()
