import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from check_private_nested import signatures


class PrivateNestedAbiTest(unittest.TestCase):
    def test_package_private_class_keeps_public_constructor_contract(self):
        output = '''final class sample.Outer$Hidden {
  public sample.Outer$Hidden(int);
    descriptor: (I)V
    flags: (0x0001) ACC_PUBLIC
  public final int getValue();
    descriptor: ()I
    flags: (0x0011) ACC_PUBLIC, ACC_FINAL
}'''
        self.assertEqual(signatures(output), sorted([
            'public sample.Outer$Hidden(int);', 'descriptor: (I)V',
            'public final int getValue();', 'descriptor: ()I',
        ]))


if __name__ == '__main__':
    unittest.main()
