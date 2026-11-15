#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
bump_version.py - Clash Mini version update tool
Usage: python scripts/bump_version.py <version>
Example: python scripts/bump_version.py 1.4.5
"""

from datetime import datetime, timezone
import io
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys

# Force stdout/stderr to use UTF-8 to avoid Windows GBK terminal encoding errors
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')


def validate_version(version: str) -> bool:
    return bool(re.fullmatch(r'\d+\.\d+\.\d+', version))


def write_utf8_lf(path: Path, content: str):
    with path.open('w', encoding='utf-8', newline='\n') as f:
        f.write(content)


def bump_package_json(root: Path, version: str):
    path = root / 'package.json'
    data = json.loads(path.read_text(encoding='utf-8'))
    old_version = data.get('version', '?')
    data['version'] = version
    write_utf8_lf(path, json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(f'  [OK] package.json          {old_version} -> {version}')


def bump_tauri_conf(root: Path, version: str):
    path = root / 'src-tauri' / 'tauri.conf.json'
    data = json.loads(path.read_text(encoding='utf-8'))
    old_version = data.get('version', '?')
    data['version'] = version
    write_utf8_lf(path, json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(f'  [OK] tauri.conf.json       {old_version} -> {version}')


def bump_cargo_toml(root: Path, version: str):
    path = root / 'src-tauri' / 'Cargo.toml'
    content = path.read_text(encoding='utf-8')

    in_package_block = False
    replaced = False
    lines = content.splitlines(keepends=True)
    new_lines = []
    old_version = '?'

    for line in lines:
        stripped = line.strip()
        if stripped == '[package]':
            in_package_block = True
        elif stripped.startswith('[') and stripped != '[package]':
            in_package_block = False

        if (
            in_package_block
            and not replaced
            and re.match(r'^version\s*=\s*"(\d+\.\d+\.\d+)"', stripped)
        ):
            m = re.match(r'^(version\s*=\s*")(\d+\.\d+\.\d+)(")', line)
            if m:
                old_version = m.group(2)
                line = m.group(1) + version + m.group(3) + '\n'
                replaced = True

        new_lines.append(line)

    if not replaced:
        print('  [WARN] Cargo.toml: [package] version field not found, skipped')
    else:
        write_utf8_lf(path, ''.join(new_lines))
        print(f'  [OK] Cargo.toml            {old_version} -> {version}')


def bump_app_update_json(root: Path, version: str):
    path = root / 'updater' / 'app-update.json'
    data = json.loads(path.read_text(encoding='utf-8'))
    old_version = data.get('version', '?')
    data['version'] = version
    data['notes'] = f'Clash Mini v{version}'
    data['pub_date'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    platforms = data.setdefault('platforms', {})
    win64 = platforms.setdefault('windows-x86_64', {})
    win64['url'] = (
        f'https://github.com/qiu-yuxiao/clash-mini/releases/download/'
        f'v{version}/Clash.Mini_{version}_x64-setup.exe'
    )
    write_utf8_lf(path, json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(f'  [OK] updater/app-update.json {old_version} -> {version}')


def format_json_files(root: Path):
    pnpm_bin = shutil.which('pnpm.cmd') or shutil.which('pnpm')
    if not pnpm_bin:
        print('  [WARN] pnpm not found in PATH, skipped biome formatting')
        return
    targets = ['package.json', 'src-tauri/tauri.conf.json', 'updater/app-update.json']
    result = subprocess.run(
        [pnpm_bin, 'exec', 'biome', 'format', '--write', *targets],
        cwd=str(root),
        capture_output=True,
        text=True,
        encoding='utf-8',
        errors='replace',
    )
    if result.returncode != 0:
        print(f'  [WARN] biome format exited {result.returncode}: {result.stderr.strip()}')
    else:
        print('  [OK] Formatted JSON targets with biome')


def main():
    if len(sys.argv) < 2:
        print('Usage: python scripts/bump_version.py <version>')
        print('Example: python scripts/bump_version.py 1.4.5')
        sys.exit(1)

    version = sys.argv[1].lstrip('v')

    if not validate_version(version):
        print(f'[ERROR] Invalid version format: "{version}"')
        print('        Expected: x.y.z (e.g. 1.4.5)')
        sys.exit(1)

    root = Path(__file__).parent.parent.resolve()

    print(f'\n>>> Bumping version to {version}')
    print(f'    Project root: {root}\n')

    try:
        bump_package_json(root, version)
        bump_tauri_conf(root, version)
        bump_cargo_toml(root, version)
        bump_app_update_json(root, version)
        format_json_files(root)
    except FileNotFoundError as e:
        print(f'\n[ERROR] File not found: {e}')
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f'\n[ERROR] JSON parse failed: {e}')
        sys.exit(1)

    print(f'\n[DONE] Version bumped to v{version}\n')


if __name__ == '__main__':
    main()
