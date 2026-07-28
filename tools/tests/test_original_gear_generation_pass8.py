import importlib.util,json,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]; S=importlib.util.spec_from_file_location('p8',ROOT/'tools/analyze-original-gear-generation-pass8.py'); M=importlib.util.module_from_spec(S); S.loader.exec_module(M)
class TestPass8(unittest.TestCase):
 def test_output(self): self.assertEqual(M.build(),json.loads(M.OUT.read_text()))
 def test_fail_closed(self):
  d=M.build(); self.assertFalse(d['directReaderBoundary']['GetGearDamageArmorAccReadPlusMinusOrRunes']); self.assertFalse(d['implementationBoundary']['liveIntegrationAllowed'])
if __name__=='__main__': unittest.main()
