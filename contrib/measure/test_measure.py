"""Tests of measure.py's arithmetic and parsing: python3 -m unittest discover contrib/measure"""

import os
import subprocess
import time
import unittest

import measure


class Summary(unittest.TestCase):
    def test_median_and_spread(self):
        self.assertEqual(
            measure.summary([0.3, 0.1, 0.2, 0.5]),
            {"n": 4, "median": 0.25, "min": 0.1, "max": 0.5},
        )


class SettledAt(unittest.TestCase):
    def test_the_first_capture_of_the_final_picture(self):
        captures = [(0.0, "a"), (0.1, "b"), (0.2, "c"), (0.3, "c"), (0.8, "c")]
        self.assertEqual(measure.settled_at(captures, after=0.0, hold=0.5), 0.2)

    def test_none_until_the_picture_held_long_enough(self):
        captures = [(0.0, "a"), (0.1, "b"), (0.2, "b")]
        self.assertIsNone(measure.settled_at(captures, after=0.0, hold=0.5))

    def test_a_picture_from_before_the_window_does_not_count(self):
        # The empty output holds still before the window maps at 0.5.
        captures = [(0.0, "empty"), (0.4, "empty"), (0.6, "app"), (1.2, "app")]
        self.assertEqual(measure.settled_at(captures, after=0.5, hold=0.5), 0.6)

    def test_a_change_starts_the_hold_again(self):
        captures = [(0.0, "a"), (0.4, "a"), (0.5, "b"), (0.7, "b")]
        self.assertIsNone(measure.settled_at(captures, after=0.0, hold=0.5))


class Smaps(unittest.TestCase):
    def test_kilobyte_fields_in_bytes(self):
        text = (
            "5a1e0000-7ffd0000 ---p 00000000 00:00 0                      [rollup]\n"
            "Rss:              135552 kB\n"
            "Pss:              113842 kB\n"
            "Private_Clean:      1000 kB\n"
            "Private_Dirty:      2000 kB\n"
        )
        fields = measure.parse_smaps_rollup(text)
        self.assertEqual(fields["Rss"], 135552 * 1024)
        self.assertEqual(fields["Pss"], 113842 * 1024)
        self.assertEqual(fields["Private_Clean"] + fields["Private_Dirty"], 3000 * 1024)


class GpuFdinfo(unittest.TestCase):
    def test_each_client_counts_once(self):
        first = "pos:\t0\ndrm-client-id:\t7\ndrm-memory-vram:\t1024 KiB\ndrm-memory-gtt:\t2048 KiB\n"
        # The same client through a second descriptor.
        again = first.replace("pos:\t0", "pos:\t1")
        other = "drm-client-id:\t9\ndrm-memory-gtt:\t512 KiB\n"
        self.assertEqual(
            measure.parse_gpu_fdinfo([first, again, other]),
            {"vram": 1024 * 1024, "gtt": (2048 + 512) * 1024},
        )

    def test_a_descriptor_without_a_client_is_skipped(self):
        self.assertEqual(measure.parse_gpu_fdinfo(["pos:\t0\nflags:\t02\n"]), {})


class Descendants(unittest.TestCase):
    def test_a_child_and_its_child_are_found(self):
        # A shell that starts a sleep: two processes under this one.
        shell = subprocess.Popen(["sh", "-c", "sleep 30 & wait"])
        try:
            for _ in range(100):
                found = measure.descendants(os.getpid())
                if len(found) >= 3:
                    break
                time.sleep(0.01)
            self.assertEqual(found[0], os.getpid())
            self.assertIn(shell.pid, found)
            self.assertEqual(len(found), 3)
        finally:
            subprocess.run(["pkill", "-P", str(shell.pid)])
            shell.wait()


if __name__ == "__main__":
    unittest.main()
