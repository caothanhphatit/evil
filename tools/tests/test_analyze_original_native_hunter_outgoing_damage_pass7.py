import json,subprocess,sys,tempfile,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
class Pass7(unittest.TestCase):
 def test_ssa_and_cap(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d)/'x.json'; subprocess.run([sys.executable,str(ROOT/'tools/analyze-original-native-hunter-outgoing-damage-pass7.py'),'--output',str(o)],check=True); x=json.loads(o.read_text())
  self.assertEqual(x['getCriticalDamageSsa']['capLiteral']['rawHex'],'6666e63f')
  self.assertEqual(x['getDamageFinalSsa'][-1],'result = truncTowardZero(H)')
  self.assertEqual(x['runtimeCompatibility'],'evidence-only-disconnected')
if __name__=='__main__': unittest.main()
