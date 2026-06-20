#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
bump_version.py - Clash Mini 版本号统一更新工具
用法: python scripts/bump_version.py <版本号>
示例: python scripts/bump_version.py 1.4.5
"""

import sys
import io
import re
import json
from pathlib import Path

# 强制 stdout/stderr 使用 UTF-8，避免 Windows GBK 终端编码错误
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

    # 只替换 [package] 块内的第一个 version = "x.y.z"
    # 避免误改 [dependencies] 中其他包的版本号
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

    version = sys.argv[1].lstrip('v')  # 兼容带 v 前缀的输入

    if not validate_version(version):
        print(f'[ERROR] Invalid version format: "{version}"')
        print('        Expected: x.y.z (e.g. 1.4.5)')
        sys.exit(1)

    # 定位项目根目录（本脚本在 scripts/ 子目录下）
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
