import struct
import tempfile
import unittest
from pathlib import Path
from import_assets import Image, prepare_numeric, sections

class ImportTests(unittest.TestCase):
    def test_raw_reads_are_bounded(self):
        with tempfile.TemporaryDirectory() as folder:
            p = Path(folder) / "raw"
            p.write_bytes(bytes(range(100)))
            image = Image(p)
            try:
                self.assertEqual(image.read(5, 3), bytes([5, 6, 7]))
                for offset, count in [(-1, 2), (99, 2), (0, -1)]:
                    with self.assertRaises(ValueError):
                        image.read(offset, count)
            finally:
                image.close()

    def test_ciso_sparse_blocks_and_boundaries(self):
        with tempfile.TemporaryDirectory() as folder:
            header = bytearray(0x8000)
            header[:4] = b"CISO"
            struct.pack_into("<I", header, 4, 16)
            header[8:11] = bytes([1, 0, 1])
            p = Path(folder) / "test.ciso"
            p.write_bytes(header + b"a" * 16 + b"b" * 16)
            image = Image(p)
            try:
                self.assertEqual(image.read(12, 24), b"a" * 4 + bytes(16) + b"b" * 4)
            finally:
                image.close()

    def test_wrong_executable_and_bad_sections_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "Unsupported executable"):
            prepare_numeric(bytes(0x200))
        with self.assertRaisesRegex(ValueError, "Truncated DOL"):
            sections(b"bad")
        dol = bytearray(0x200)
        struct.pack_into(">I", dol, 0, 0x100)
        struct.pack_into(">I", dol, 0x48, 0x80000000)
        struct.pack_into(">I", dol, 0x90, 0x101)
        with self.assertRaisesRegex(ValueError, "bounds"):
            sections(dol)
        struct.pack_into(">I", dol, 0x90, 0x100)
        self.assertEqual(sections(dol), [(0, 0x100, 0x80000000, 0x100)])

if __name__ == "__main__":
    unittest.main()
