#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
bump_version.py - Clash Mini version update tool
Usage: python scripts/bump_version.py <version>
Example: python scripts/bump_version.py 1.4.5
"""

import sys
import io
import re
import json
from pathlib import Path

# Force stdout/stderr to use UTF-8 to avoid Windows GBK terminal encoding errors
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')


def validate_version(version: str) -> bool:
    return bool(re.fullmatch(r'\d+\.\d+\.\d+', version))


def bump_package_json(root: Path, version: str):
    path = root / 'package.json'
    content = path.read_text(encoding='utf-8')
    data = json.loads(content)
    old_version = data.get('version', '?')
    data['version'] = version
    new_content = json.dumps(data, indent=2, ensure_ascii=False) + '\n'
    path.write_text(new_content, encoding='utf-8')
    print(f'  [OK] package.json          {old_version} -> {version}')


def bump_tauri_conf(root: Path, version: str):
    path = root / 'src-tauri' / 'tauri.conf.json'
    content = path.read_text(encoding='utf-8')
    data = json.loads(content)
    old_version = data.get('version', '?')
    data['version'] = version
    new_content = json.dumps(data, indent=2, ensure_ascii=False) + '\n'
    path.write_text(new_content, encoding='utf-8')
    print(f'  [OK] tauri.conf.json       {old_version} -> {version}')


def bump_cargo_toml(root: Path, version: str):
    path = root / 'src-tauri' / 'Cargo.toml'
    content = path.read_text(encoding='utf-8')

    # Only replace the first version = "x.y.z" inside the [package] block
    # Avoid accidentally changing other packages' versions in [dependencies]
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

        if in_package_block and not replaced and re.match(r'^version\s*=\s*"(\d+\.\d+\.\d+)"', stripped):
            m = re.match(r'^(version\s*=\s*")(\d+\.\d+\.\d+)(")', line)
            if m:
                old_version = m.group(2)
                line = m.group(1) + version + m.group(3) + '\n'
                replaced = True

        new_lines.append(line)

    if not replaced:
        print(f'  [WARN] Cargo.toml: [package] version field not found, skipped')
    else:
        path.write_text(''.join(new_lines), encoding='utf-8')
        print(f'  [OK] Cargo.toml            {old_version} -> {version}')


def main():
    if len(sys.argv) < 2:
        print('Usage: python scripts/bump_version.py <version>')
        print('Example: python scripts/bump_version.py 1.4.5')
        sys.exit(1)

    version = sys.argv[1].lstrip('v')  # Support input with 'v' prefix

    if not validate_version(version):
        print(f'[ERROR] Invalid version format: "{version}"')
        print('        Expected: x.y.z (e.g. 1.4.5)')
        sys.exit(1)

    # Locate project root directory (this script is in the scripts/ subdirectory)
    root = Path(__file__).parent.parent.resolve()

    print(f'\n>>> Bumping version to {version}')
    print(f'    Project root: {root}\n')

    try:
        bump_package_json(root, version)
        bump_tauri_conf(root, version)
        bump_cargo_toml(root, version)
    except FileNotFoundError as e:
        print(f'\n[ERROR] File not found: {e}')
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f'\n[ERROR] JSON parse failed: {e}')
        sys.exit(1)

    print(f'\n[DONE] Version bumped to v{version}\n')


if __name__ == '__main__':
    main()
