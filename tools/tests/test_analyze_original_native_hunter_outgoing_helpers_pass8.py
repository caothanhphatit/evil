import json,subprocess,sys,tempfile,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
class Pass8(unittest.TestCase):
 def test_helpers(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d)/'x';subprocess.run([sys.executable,str(ROOT/'tools/analyze-original-native-hunter-outgoing-helpers-pass8.py'),'--output',str(o)],check=True);x=json.loads(o.read_text())
  self.assertEqual(x['methods']['getSlayerDamage']['size'],3512);self.assertEqual(x['methods']['getRiftNpcBuffDamage']['size'],592);self.assertEqual(x['rift']['equation'],'result += decoded nested integer * 0.0001f');self.assertEqual(x['slayer']['raceBranches']['5']['gearPropertyIndex'],45)
if __name__=='__main__':unittest.main()
