import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[1] / 'tools'
sys.path.insert(0, str(TOOLS))
spec = importlib.util.spec_from_file_location('listener_check', TOOLS / 'check_androidbible_class_listener.py')
listener = importlib.util.module_from_spec(spec)
spec.loader.exec_module(listener)


def fnv(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
    return value


class OwnershipTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name)
        self.source = self.output / 'Input.kt'
        self.artifact = 'classes/Body.class'
        path = self.output / self.artifact
        path.parent.mkdir()
        path.write_bytes(b'emitted output')
        (self.output / '.korrust').mkdir()
        self.write_index()

    def write_index(self, owner=None, artifact=None, digest=None, duplicate=False):
        owner = owner or self.source
        artifact = artifact or self.artifact
        digest = fnv((self.output / self.artifact).read_bytes()) if digest is None else digest
        body = f'{str(owner).encode().hex()}\t{artifact.encode().hex()}\t{digest:016x}\n'
        if duplicate:
            body += body
        (self.output / '.korrust/index').write_text(f'korrust-output-v1\t{fnv(body.encode()):016x}\n' + body)

    def test_all_emitted_outputs_owned_by_original_source(self):
        self.assertEqual(set(listener.ownership(self.output, self.source)), {self.artifact})

    def test_ownership_rejects_content_tampering(self):
        self.write_index(digest=0)
        with self.assertRaisesRegex(ValueError, 'content changed'):
            listener.ownership(self.output, self.source)

    def test_ownership_rejects_another_source(self):
        self.write_index(owner=self.output / 'Other.kt')
        with self.assertRaisesRegex(ValueError, 'source ownership'):
            listener.ownership(self.output, self.source)

    def test_ownership_rejects_unowned_output(self):
        (self.output / 'Extra.class').write_bytes(b'unowned')
        with self.assertRaisesRegex(ValueError, 'Missing or extra'):
            listener.ownership(self.output, self.source)

    def test_ownership_rejects_duplicate_records(self):
        self.write_index(duplicate=True)
        with self.assertRaisesRegex(ValueError, 'duplicated source ownership'):
            listener.ownership(self.output, self.source)

    def test_ownership_rejects_index_tampering(self):
        path = self.output / '.korrust/index'
        path.write_text(path.read_text().replace('korrust-output-v1', 'invalid'))
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            listener.ownership(self.output, self.source)


if __name__ == '__main__':
    unittest.main()
