import struct
import unittest
from types import SimpleNamespace

from tools.scene_evidence_lib import color, mono_behaviour_header, quaternion, vector2, vector3


class SceneEvidenceHelpersTest(unittest.TestCase):
    def test_vectors_quaternion_and_color_are_json_ready(self):
        self.assertEqual(vector2(SimpleNamespace(x=1, y=2)), {"x": 1.0, "y": 2.0})
        self.assertEqual(vector3(SimpleNamespace(x=1, y=2, z=3)), {"x": 1.0, "y": 2.0, "z": 3.0})
        self.assertEqual(quaternion(SimpleNamespace(x=0, y=0, z=0, w=1)), {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0})
        self.assertEqual(color(SimpleNamespace(r=1, g=.5, b=0, a=1)), {"r": 1.0, "g": .5, "b": 0.0, "a": 1.0})

    def test_mono_behaviour_header_decodes_pptrs(self):
        payload = bytearray(32)
        struct.pack_into("<iq", payload, 0, 0, 248)
        payload[12] = 1
        struct.pack_into("<iq", payload, 16, 1, 4391)
        self.assertEqual(mono_behaviour_header(payload), {
            "gameObjectFileId": 0, "gameObjectPathId": 248, "enabled": True,
            "scriptFileId": 1, "scriptPathId": 4391,
        })

    def test_short_mono_behaviour_header_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "32-byte header"):
            mono_behaviour_header(b"short")


if __name__ == "__main__":
    unittest.main()
