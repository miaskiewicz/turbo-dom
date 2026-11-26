import importlib.util
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

spec = importlib.util.spec_from_file_location('css_shared', Path(__file__).with_name('css-shared.py'))
shared = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shared)


class SharedPackaging(unittest.TestCase):
    def populate(self, root):
        (root / 'package.json').write_text('{"version":"0.8.7"}')
        for directory, glue in [('pkg-css', "const wasmPath = `${__dirname}/turbo_dom_css_bg.wasm`;"),
                                ('pkg-css-web', "new URL('turbo_dom_css_bg.wasm', import.meta.url)")]:
            folder = root / directory
            folder.mkdir()
            (folder / 'package.json').write_text('{"version":"0.8.7"}')
            for suffix, content in [('.js', b'/* @ts-self-types="./turbo_dom_css.d.ts" */\n' + glue.encode()), ('.d.ts', directory.encode()),
                                    ('_bg.wasm', b'\0asm\1\0\0\0body'), ('_bg.wasm.d.ts', b'types')]:
                (folder / f'turbo_dom_css{suffix}').write_bytes(content)

    def test_shared_bytes_and_separate_exact_declarations(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.populate(root)
            shared.assemble(root)
            self.assertEqual(len(list(root.rglob('*.wasm'))), 1)
            self.assertEqual((root / 'css-wasm/turbo_dom_css.node.d.cts').read_bytes(), b'pkg-css')
            self.assertEqual((root / 'css-wasm/turbo_dom_css.web.d.mts').read_bytes(), b'pkg-css-web')
            self.assertTrue(shared.check(root))

    def test_incompatible_outputs_refused_before_any_mutation(self):
        for kind in ['wasm', 'declarations', 'snippet', 'import', 'path', 'version']:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.populate(root)
                web = root / 'pkg-css-web'
                if kind == 'wasm': (web / 'turbo_dom_css_bg.wasm').write_bytes(b'\0asm\1\0\0\0different')
                if kind == 'declarations': (web / 'turbo_dom_css_bg.wasm.d.ts').write_text('different')
                if kind == 'snippet': (web / 'snippets').mkdir()
                if kind == 'import':
                    with (web / 'turbo_dom_css.js').open('a') as f: f.write(";import x from './other.js'")
                if kind == 'path': (web / 'turbo_dom_css.js').write_text("new URL('other.wasm', import.meta.url)")
                if kind == 'version': (web / 'package.json').write_text('{"version":"0.8.6"}')
                with self.assertRaises(ValueError): shared.assemble(root)
                self.assertFalse((root / 'css-wasm').exists())
                self.assertTrue((root / 'pkg-css/turbo_dom_css_bg.wasm').exists())

    def test_changed_manifest_glue_typings_duplicate_and_snippets_fail(self):
        for kind in ['manifest', 'glue', 'typing', 'duplicate', 'snippet']:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.populate(root)
                shared.assemble(root)
                if kind == 'manifest': (root / 'css-wasm/package.json').write_text('{"version":"0.8.7","generated":{}}')
                if kind == 'glue': (root / 'css-wasm/turbo_dom_css.node.cjs').write_text('changed')
                if kind == 'typing': (root / 'pkg-css/turbo_dom_css.d.ts').write_text('changed')
                if kind == 'duplicate': (root / 'pkg-css/turbo_dom_css_bg.wasm').write_bytes(b'\0asm\1\0\0\0')
                if kind == 'snippet': (root / 'css-wasm/snippets').mkdir()
                with self.assertRaises(ValueError): shared.check(root)



    def test_comment_transform_is_exact_and_executable_body_unchanged(self):
        header = b'/* @ts-self-types="./turbo_dom_css.d.ts" */'
        body = b'\nconst code = 1;'
        result, receipt = shared.type_comment(header + body, 'node')
        self.assertEqual(result.split(b'*/', 1)[1], body)
        shared.check_type_comment(result, 'node', receipt)
        for source in [body, header + header + body, header.replace(b'.d.ts', b'.other.d.ts') + body]:
            with self.assertRaises(ValueError): shared.type_comment(source, 'node')

    def test_failed_multi_directory_replacement_rolls_back(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.populate(root)
            original = (root / 'pkg-css/turbo_dom_css.js').read_bytes()
            rename = Path.rename
            def fail_web(source, destination):
                if source.name == 'pkg-css-web' and 'css-shared-' in str(source.parent):
                    raise OSError('injected destination failure')
                return rename(source, destination)
            with patch.object(Path, 'rename', fail_web), self.assertRaises(OSError): shared.assemble(root)
            self.assertEqual((root / 'pkg-css/turbo_dom_css.js').read_bytes(), original)
            self.assertTrue((root / 'pkg-css-web/turbo_dom_css_bg.wasm').exists())
            self.assertFalse((root / 'css-wasm').exists())


if __name__ == '__main__': unittest.main()
