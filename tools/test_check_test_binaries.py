"""Regression checks for disabled autodiscovery and compiled inventory drift."""
from pathlib import Path
import tempfile
import unittest

import check_test_binaries as guard


class TestBinaryGuard(unittest.TestCase):
    def fixture(self, root):
        (root / 'tests/common').mkdir(parents=True)
        (root / 'tests/group').mkdir()
        (root / 'Cargo.toml').write_text('''[package]
name = "fixture"
autotests = false
[[test]]
name = "group"
path = "tests/group/main.rs"
''')
        (root / 'tests/group/main.rs').write_text('''
#[path = "../cases.rs"]
mod cases;
''')
        (root / 'tests/cases.rs').write_text('''
#[path = "common/support.rs"]
mod support;
// mod ghost;
const TEXT: &str = "mod ghost; #[test] fn ghost() {}";
const CONTINUED: &str = "first \\\n+second { mod ghost; }";
#[test]
fn first() { let json = r#"{ mod ghost; }"#; }
mod nested {
    #[test]
    fn second() {}
}
part_cases!(parts; alpha => "a.step", beta => "b.step");
pmi_cases!(pmi);
''')
        (root / 'tests/common/support.rs').write_text('''
macro_rules! pmi_files {
    ($emit:ident) => { $emit!(one => "one.step", two => "two.step"); };
}
''')

    def test_walks_modules_and_generated_tests_without_reading_comments_or_strings(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.fixture(root)
            expected, reached = guard.inventory(root)
            self.assertEqual(expected, {'group': {
                'cases::first', 'cases::nested::second', 'cases::parts::alpha',
                'cases::parts::beta', 'cases::pmi::one', 'cases::pmi::two'}})
            self.assertEqual(len(reached), 3)

    def test_new_unreferenced_file_fails_even_in_a_nested_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.fixture(root)
            (root / 'tests/common/forgotten.rs').write_text('#[test] fn lost() {}')
            with self.assertRaisesRegex(ValueError, 'orphan.*forgotten.rs'):
                guard.inventory(root)

    def test_count_drop_or_replacement_fails(self):
        expected = {'group': {'cases::first', 'cases::second'}}
        def listing(names):
            return {'rust-suites': {'id': {
                'kind': 'test', 'binary-name': 'group',
                'testcases': {name: {} for name in names}}}}
        guard.check_list(expected, listing(expected['group']))
        for names in ({'cases::first'}, {'cases::first', 'cases::replacement'}):
            with self.assertRaisesRegex(ValueError, 'missing=.*cases::second'):
                guard.check_list(expected, listing(names))

    def test_repository_and_ci_keep_both_guards_enabled(self):
        expected, _ = guard.inventory()
        self.assertEqual(set(expected), {
            'analysis', 'dimensions', 'drawing', 'documents', 'audits', 'interface',
            'issue_drafting', 'issue_trust'})
        workflow = (guard.ROOT / '.github/workflows/ci.yml').read_text()
        lint, rest = workflow.split('  lint:', 1)[1].split('  build:', 1)
        build = rest.split('  test:', 1)[0]
        self.assertIn('python3 tools/check_test_binaries.py', lint)
        self.assertIn('--ignore-default-filter --message-format json', build)
        self.assertIn('python3 tools/check_test_binaries.py --nextest-list', build)
        manifest = guard.tomllib.loads((guard.ROOT / 'Cargo.toml').read_text())
        self.assertFalse(manifest['package']['autotests'])


if __name__ == '__main__':
    unittest.main()
