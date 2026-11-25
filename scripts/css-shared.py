#!/usr/bin/env python3
"""Co-locate CSS glue with audited type-comment-only edits; refuse unreviewed module layouts."""
import argparse
import hashlib
import json
import re
import shutil
import tempfile
from pathlib import Path

STEM = 'turbo_dom_css'
NODE_WRAPPER = "module.exports = require('../css-wasm/turbo_dom_css.node.cjs');\n"
WEB_WRAPPER = "export * from '../css-wasm/turbo_dom_css.web.mjs';\nexport { default } from '../css-wasm/turbo_dom_css.web.mjs';\n"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def inputs(root):
    result = {}
    for directory in ('pkg-css', 'pkg-css-web'):
        folder = root / directory
        if (folder / 'snippets').exists():
            raise ValueError('CSS shared assembly refuses unreviewed generated snippets')
        package = json.loads((folder / 'package.json').read_text())
        if package.get('version') != json.loads((root / 'package.json').read_text())['version']:
            raise ValueError('CSS generated package version mismatch')
        result[directory] = {suffix: (folder / f'{STEM}{suffix}').read_bytes()
                             for suffix in ('.js', '.d.ts', '_bg.wasm', '_bg.wasm.d.ts')}
    node, web = result.values()
    if node['_bg.wasm'] != web['_bg.wasm'] or not node['_bg.wasm'].startswith(b'\0asm\1\0\0\0'):
        raise ValueError('CSS Node/web WASM differs or has an invalid header')
    if node['_bg.wasm.d.ts'] != web['_bg.wasm.d.ts']:
        raise ValueError('CSS Node/web low-level WASM declarations differ')
    validate_glue(node['.js'], web['.js'])
    return result

def validate_glue(node, web):
    expected_node = "const wasmPath = `${__dirname}/turbo_dom_css_bg.wasm`;"
    expected_web = "new URL('turbo_dom_css_bg.wasm', import.meta.url)"
    if expected_node not in node.decode() or expected_web not in web.decode():
        raise ValueError('CSS generated sibling WASM resolution changed')
    for source in (node.decode(), web.decode()):
        if re.search(r'\b(?:import\s*\(|from\s*[\'\"]|require\s*\(\s*[\'\"](?!fs[\'\"]))', source):
            raise ValueError('CSS generated imports require a new shared-module review')


def type_comment(source, flavor):
    original = b'/* @ts-self-types="./turbo_dom_css.d.ts" */'
    if source.count(b'@ts-self-types') != 1 or source.count(original) != 1 or not source.startswith(original):
        raise ValueError('CSS generated self-types comment drift')
    extension = 'cts' if flavor == 'node' else 'mts'
    comment = f'/* @ts-self-types="./turbo_dom_css.{flavor}.d.{extension}" */'.encode()
    output = source.replace(original, comment, 1)
    if source[len(original):] != output[len(comment):]:
        raise ValueError('CSS generated executable body changed')
    return output, {'originalSha256': digest(source), 'outputSha256': digest(output),
                    'runtimeBodySha256': digest(source[len(original):])}

def check_type_comment(source, flavor, receipt):
    extension = 'cts' if flavor == 'node' else 'mts'
    comment = f'/* @ts-self-types="./turbo_dom_css.{flavor}.d.{extension}" */'.encode()
    if not source.startswith(comment) or source.count(b'@ts-self-types') != 1:
        raise ValueError('CSS derived self-types comment drift')
    original = b'/* @ts-self-types="./turbo_dom_css.d.ts" */' + source[len(comment):]
    _, expected = type_comment(original, flavor)
    if receipt != expected:
        raise ValueError('CSS generated comment-only transform receipt mismatch')

def assemble(root):
    data = inputs(root)
    shared = root / 'css-wasm'
    with tempfile.TemporaryDirectory(prefix='css-shared-', dir=root) as temporary:
        stage = Path(temporary) / 'new'
        stage.mkdir()
        node, web = data.values()
        node_glue, node_receipt = type_comment(node['.js'], 'node')
        web_glue, web_receipt = type_comment(web['.js'], 'web')
        outputs = {'turbo_dom_css.node.cjs': node_glue, 'turbo_dom_css.web.mjs': web_glue,
                   'turbo_dom_css_bg.wasm': node['_bg.wasm'],
                   'turbo_dom_css_bg.wasm.d.ts': node['_bg.wasm.d.ts'],
                   'turbo_dom_css.web.d.mts': web['.d.ts'],
                   'turbo_dom_css.node.d.cts': node['.d.ts']}
        for name, content in outputs.items():
            (stage / name).write_bytes(content)
        version = json.loads((root / 'package.json').read_text())['version']
        manifest = {'version': version, 'private': True,
                    'generated': {name: digest(content) for name, content in outputs.items()},
                    'commentTransforms': {'node': node_receipt, 'web': web_receipt}}
        (stage / 'package.json').write_text(json.dumps(manifest, indent=2) + '\n')
        replacements = {'css-wasm': stage}
        for directory, wrapper in [('pkg-css', NODE_WRAPPER), ('pkg-css-web', WEB_WRAPPER)]:
            folder = Path(temporary) / directory
            shutil.copytree(root / directory, folder)
            (folder / f'{STEM}.js').write_text(wrapper)
            (folder / f'{STEM}_bg.wasm').unlink()
            (folder / f'{STEM}_bg.wasm.d.ts').unlink()
            replacements[directory] = folder
        replace_packages(root, replacements, Path(temporary))
    return check(root)


def replace_packages(root, replacements, temporary):
    completed = []
    try:
        for name, stage in replacements.items():
            destination, backup = root / name, temporary / f'previous-{name}'
            existed = destination.exists()
            if existed: destination.rename(backup)
            completed.append((destination, backup, existed))
            stage.rename(destination)
    except BaseException:
        for destination, backup, existed in reversed(completed):
            if destination.exists(): shutil.rmtree(destination)
            if existed: backup.rename(destination)
        raise

def check(root):
    shared = root / 'css-wasm'
    manifest = json.loads((shared / 'package.json').read_text())
    version = json.loads((root / 'package.json').read_text())['version']
    if manifest.get('version') != version:
        raise ValueError('CSS shared package version mismatch')
    expected_names = {f'{STEM}{suffix}' for suffix in ('.node.cjs', '.web.mjs', '_bg.wasm', '_bg.wasm.d.ts', '.web.d.mts', '.node.d.cts')}
    if set(manifest.get('generated', {})) != expected_names:
        raise ValueError('CSS shared generated file set mismatch')
    if {p.name for p in shared.iterdir()} != expected_names | {'package.json'}:
        raise ValueError('CSS shared unexpected file or snippet')
    records = []
    for name, expected in manifest['generated'].items():
        path = shared / name
        if digest(path.read_bytes()) != expected:
            raise ValueError(f'CSS shared generated artifact changed: {name}')
        records.append(path.relative_to(root))
    for flavor, extension in [('node', 'cjs'), ('web', 'mjs')]:
        check_type_comment((shared / f'{STEM}.{flavor}.{extension}').read_bytes(), flavor,
                           manifest['commentTransforms'][flavor])
    validate_glue((shared / f'{STEM}.node.cjs').read_bytes(), (shared / f'{STEM}.web.mjs').read_bytes())
    for directory, wrapper in [('pkg-css', NODE_WRAPPER), ('pkg-css-web', WEB_WRAPPER)]:
        folder = root / directory
        if (folder / f'{STEM}.js').read_text() != wrapper:
            raise ValueError('CSS forwarding module changed')
        if list(folder.glob('*.wasm')) or (folder / 'snippets').exists():
            raise ValueError('CSS duplicate WASM or unreviewed snippets remain')
        package = json.loads((folder / 'package.json').read_text())
        if package.get('version') != version:
            raise ValueError('CSS wrapper version mismatch')
        declaration = 'turbo_dom_css.node.d.cts' if directory == 'pkg-css' else 'turbo_dom_css.web.d.mts'
        if (folder / f'{STEM}.d.ts').read_bytes() != (shared / declaration).read_bytes():
            raise ValueError('CSS wrapper declarations differ from generated declarations')
        records.extend(folder.joinpath(name).relative_to(root) for name in ['package.json', f'{STEM}.js', f'{STEM}.d.ts'])
    records.append(Path('css-wasm/package.json'))
    return records


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['assemble', 'check'])
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    print(json.dumps([str(path) for path in (assemble(args.root) if args.command == 'assemble' else check(args.root))], indent=2))
