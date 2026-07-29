import unittest
import importlib.util
from pathlib import Path

_MODULE_PATH = Path(__file__).parents[1] / "runtime" / "capture-android-process-methods.py"
_SPEC = importlib.util.spec_from_file_location("capture_android_process_methods", _MODULE_PATH)
_MODULE = importlib.util.module_from_spec(_SPEC)
assert _SPEC.loader is not None
_SPEC.loader.exec_module(_MODULE)
parse_maps = _MODULE.parse_maps
resolve_module_base = _MODULE.resolve_module_base


class CaptureAndroidProcessMethodsTest(unittest.TestCase):
    def test_parse_maps_keeps_package_executable_ranges(self):
        mappings = parse_maps(
            "\n".join([
                "1000-2000 r-xp 00001000 00:00 1 /system/lib.so",
                "7000-9000 r-xp 00003000 00:00 2 /data/app/split_config.arm64_v8a.apk",
                "9000-a000 rw-p 00005000 00:00 2 /data/app/split_config.arm64_v8a.apk",
            ]),
            "split_config.arm64_v8a.apk",
        )
        self.assertEqual(mappings, [
            (0x7000, 0x9000, 0x3000, "/data/app/split_config.arm64_v8a.apk"),
            (0x9000, 0xA000, 0x5000, "/data/app/split_config.arm64_v8a.apk"),
        ])

    def test_resolve_module_base_requires_all_offsets(self):
        mappings = [
            (0x7000, 0x9000, 0x3000, "split_config.arm64_v8a.apk"),
            (0x9000, 0xA000, 0x5000, "split_config.arm64_v8a.apk"),
        ]
        self.assertEqual(resolve_module_base(mappings, [0x4000, 0x5000]), 0x4000)
        with self.assertRaisesRegex(RuntimeError, "Unable to resolve"):
            resolve_module_base(mappings, [0x4000, 0x9000])


if __name__ == "__main__":
    unittest.main()
