"""Synthetic tests; retail acceptance evidence lives outside Git."""
import struct
import unittest

import xbox_save_probe as probe


class XboxSaveProbeTests(unittest.TestCase):
    def setUp(self):
        self.key = bytes(range(16))
        data = bytearray(probe.SAVE_SIZE)
        for mirror in range(2):
            base = mirror * probe.RECORD_SIZE
            struct.pack_into("<I", data, base, 4)
            for box in range(4):
                slot = base + 4 + box * probe.SLOT_SIZE
                struct.pack_into("<II", data, slot, 1, box + 10)
                struct.pack_into("<I", data, slot + 0x60, box + 100)
        # Start with signed records, without changing any fields.
        for mirror in range(2):
            base = mirror * probe.RECORD_SIZE
            data[base + probe.SIGNED_SIZE:base + probe.RECORD_SIZE] = probe.hmac.digest(
                self.key, data[base:base + probe.SIGNED_SIZE], "sha1")
        self.data = bytes(data)

    def test_selected_box_only_and_both_mirrors(self):
        edited = probe.edit(self.data, self.key, 3, "XBEDIT", 123456)
        report = probe.inspect(edited, self.key)
        self.assertTrue(all(probe.verify(edited, self.key)))
        self.assertTrue(report["mirrors_identical"])
        allowed = set()
        for mirror in range(2):
            base = mirror * probe.RECORD_SIZE
            slot = base + 4 + 2 * probe.SLOT_SIZE
            allowed.update(range(slot + 0x28, slot + 0x3A))
            allowed.update(range(slot + 0x60, slot + 0x64))
            allowed.update(range(base + probe.SIGNED_SIZE, base + probe.RECORD_SIZE))
            boxes = report["records"][mirror]["boxes"]
            self.assertEqual(boxes[2]["bit"], 123456)
            self.assertEqual(boxes[2]["name"], "ＸＢＥＤＩＴ")
            self.assertEqual([b["bit"] for b in boxes if b["box"] != 3], [100, 101, 103])
        changed = {i for i, (a, b) in enumerate(zip(self.data, edited)) if a != b}
        self.assertLessEqual(changed, allowed)

    def test_corrupt_mirror_is_not_silently_repaired(self):
        damaged = bytearray(self.data)
        damaged[probe.RECORD_SIZE + 123] ^= 1
        self.assertEqual(probe.verify(damaged, self.key), [True, False])
        with self.assertRaises(ValueError):
            probe.edit(damaged, self.key, 1, bit=1)

    def test_bad_size_box_and_name_rejected(self):
        with self.assertRaises(ValueError):
            probe.verify(self.data[:-1], self.key)
        for box in (0, 5):
            with self.assertRaises(ValueError):
                probe.edit(self.data, self.key, box, bit=1)
        for name in ("", "123456789", "🙂"):
            with self.assertRaises(ValueError):
                probe.edit(self.data, self.key, 1, name=name)


if __name__ == "__main__":
    unittest.main()
