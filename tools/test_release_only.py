import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('release_only', ROOT / 'tools/release_only.py')
release_only = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release_only)


class ReleaseOnly(unittest.TestCase):
    def repo(self, before, after):
        d = tempfile.TemporaryDirectory()
        self.addCleanup(d.cleanup)
        run = lambda *a: subprocess.run(['git', '-C', d.name, *a], check=True, capture_output=True)
        run('init', '-q')
        run('config', 'user.email', 't@example.com')
        run('config', 'user.name', 't')
        for files in (before, after):
            for name, text in files.items():
                p = Path(d.name, name)
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text(text)
            run('add', '-A')
            run('commit', '-qm', 'c')
        cwd = Path.cwd()
        import os
        os.chdir(d.name)
        self.addCleanup(os.chdir, cwd)
        return 'HEAD~1', 'HEAD'

    base = {'Cargo.toml': '[package]\nname = "x"\nversion = "0.1.0-alpha.4"\n',
            'CHANGELOG.md': '# Changes\n', 'src/lib.rs': 'pub fn f() {}\n'}

    def test_version_bump_and_notes_are_release_only(self):
        after = {'Cargo.toml': self.base['Cargo.toml'].replace('alpha.4', 'alpha.5'),
                 'CHANGELOG.md': '# Changes\n\n## 0.1.0-alpha.5\n', 'docs/notes.md': 'n\n'}
        self.assertTrue(release_only.release_only(*self.repo(self.base, after))[0])

    def test_source_change_is_not(self):
        after = {'src/lib.rs': 'pub fn g() {}\n'}
        self.assertFalse(release_only.release_only(*self.repo(self.base, after))[0])

    def test_other_cargo_edit_is_not(self):
        after = {'Cargo.toml': self.base['Cargo.toml'] + '[dependencies]\nserde = "1"\n'}
        self.assertFalse(release_only.release_only(*self.repo(self.base, after))[0])

    def test_two_different_versions_are_not(self):
        after = {'Cargo.toml': self.base['Cargo.toml'].replace('alpha.4', 'alpha.5'),
                 'crates/w/Cargo.toml': 'version = "9.9.9"\n'}
        before = dict(self.base, **{'crates/w/Cargo.toml': 'version = "1.0.0"\n'})
        self.assertFalse(release_only.release_only(*self.repo(before, after))[0])


if __name__ == '__main__':
    unittest.main()
