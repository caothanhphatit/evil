import json,subprocess,sys,tempfile,unittest
from pathlib import Path
R=Path(__file__).resolve().parents[2]
class T(unittest.TestCase):
 def test_tree(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d)/'x';subprocess.run([sys.executable,str(R/'tools/analyze-original-native-hunter-d8-d10-pass14.py'),'--output',str(o)],check=True);x=json.loads(o.read_text())
  self.assertEqual(x['method']['size'],9496);self.assertIn('CalcDamage / CalcAttackSpeed',x['arg1Tree']['arg1True']);self.assertEqual(len(x['jobSubJobSelector']['observedSpecialPairs']),4);self.assertEqual(x['staticDouble']['raw'],'7b14ae47e17a843f');self.assertEqual(x['staticDouble']['float64'],0.01);self.assertIn('no integer rounding',x['jobTrait5Augmentation']['castsRounding'])
if __name__=='__main__':unittest.main()
