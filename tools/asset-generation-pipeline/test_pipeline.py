import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parent


class PipelineTest(unittest.TestCase):
    def test_catalog_has_forty_bilingual_weapons(self):
        catalog = json.loads((ROOT / "profiles/weapons/catalog.json").read_text(encoding="utf-8"))
        self.assertEqual(len(catalog["weapons"]), 40)
        self.assertEqual({row["unlockLevel"] for row in catalog["weapons"]}, set(range(0, 800, 100)))
        self.assertTrue(all(row["en"] and row["vi"] for row in catalog["weapons"]))

    def test_reference_builder_uses_existing_source_icons(self):
        source = ROOT.parents[1] / "apps/web/public/content/releases/evil-hunter-1.411/gear-icons"
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "reference.png"
            subprocess.run(
                ["python3", str(ROOT / "pipeline.py"), "--profile", "weapons", "reference", "--source", str(source), "--output", str(output)],
                check=True,
                capture_output=True,
                text=True,
            )
            with Image.open(output) as image:
                self.assertEqual(image.size, (1008, 720))


if __name__ == "__main__":
    unittest.main()
