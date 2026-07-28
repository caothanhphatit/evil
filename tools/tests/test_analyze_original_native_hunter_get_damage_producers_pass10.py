import json,subprocess,sys,tempfile,unittest
from pathlib import Path
R=Path(__file__).resolve().parents[2]
class P10(unittest.TestCase):
 def test_producers(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d)/'x';subprocess.run([sys.executable,str(R/'tools/analyze-original-native-hunter-get-damage-producers-pass10.py'),'--output',str(o)],check=True);x=json.loads(o.read_text())
  self.assertEqual(x['method']['size'],9496);self.assertEqual(x['producers']['D11']['default'],'1.0');self.assertEqual(x['producers']['S9_final']['selector'],'HunterData.job@0x20');self.assertIn('bypasses',x['booleanArguments']['arg2_w2_saved_w22'])
if __name__=='__main__':unittest.main()
