import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from check_inner_classes import signatures


class ConsumerAbiPolicyTest(unittest.TestCase):
    def test_compiler_private_access_helpers_require_synthetic_static_modifiers(self):
        text = '''public final class Sample {
  public static final int access$hidden(Sample);
    descriptor: (LSample;)I
    flags: (0x1019) ACC_PUBLIC, ACC_STATIC, ACC_FINAL, ACC_SYNTHETIC
}'''
        self.assertEqual(signatures(text), ['public final class Sample {'])

    def test_source_members_constructors_fields_default_helpers_and_bridges_are_retained(self):
        for header, flags, descriptor in [
            ('public int access$source();', 'ACC_PUBLIC', '()I'),
            ('public static int source$default();', 'ACC_PUBLIC, ACC_STATIC, ACC_SYNTHETIC', '()I'),
            ('public Sample(Sample);', 'ACC_PUBLIC, ACC_SYNTHETIC', '(LSample;)V'),
            ('public static int access$field;', 'ACC_PUBLIC, ACC_STATIC, ACC_SYNTHETIC', 'I'),
            ('public int access$bridge();', 'ACC_PUBLIC, ACC_SYNTHETIC, ACC_BRIDGE', '()I'),
        ]:
            with self.subTest(header=header):
                self.assertEqual(signatures(header + '\n descriptor: ' + descriptor + '\n flags: ' + flags), sorted([header, 'descriptor: ' + descriptor]))


if __name__ == '__main__':
    unittest.main()
