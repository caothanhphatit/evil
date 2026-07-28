#!/usr/bin/env python3
"""Normalize exact Slayer and Rift outgoing helper bodies without semantics guesses."""
from __future__ import annotations
import argparse,hashlib,json,struct
from pathlib import Path
from capstone import Cs,CS_ARCH_ARM64,CS_MODE_ARM
ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/'reverse-engineering/evidence/original-native-hunter-outgoing-helper-methods-api35-v1.json'
OUT=ROOT/'reverse-engineering/evidence/original-native-hunter-outgoing-helpers-pass8.json'
IMAGE=Path('/tmp/evil-libil2cpp-memory.bin')
def method(p,n): return next(x for x in p['methods'] if x['methodName']==n)
def decode(m):
 c=m['candidates'][0]; b=bytes.fromhex(c['codeHex']); base=int(c['moduleOffset'],16); ins={i.address-base:(i.mnemonic,i.op_str) for i in Cs(CS_ARCH_ARM64,CS_MODE_ARM).disasm(b,base)}; return c,b,ins
def req(i,o,m,a):
 if i.get(o)!=(m,a): raise ValueError(f'{o:x}: {i.get(o)} != {(m,a)}')
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--source',type=Path,default=SRC);ap.add_argument('--memory-image',type=Path,default=IMAGE);ap.add_argument('--output',type=Path,default=OUT);a=ap.parse_args();p=json.loads(a.source.read_text())['record']['payload'];sc,sb,si=decode(method(p,'getSlayerDamage'));rc,rb,ri=decode(method(p,'getRiftNpcBuffDamage'))
 for x in [(si,0x224,'fmul','s0, s0, s1'),(si,0x228,'fadd','s0, s8, s0'),(si,0x3e0,'fmul','s0, s0, s10'),(si,0x3e4,'fadd','s0, s8, s0'),(si,0x6bc,'fmul','s0, s0, s10'),(si,0x6c0,'fadd','s0, s8, s0'),(si,0xa7c,'fmul','s0, s0, s1'),(si,0xa80,'fadd','s0, s8, s0'),(si,0xc1c,'fmul','s0, s0, s9'),(si,0xc20,'fadd','s0, s8, s0')]: req(*x)
 for x in [(ri,0x218,'fmul','s0, s0, s1'),(ri,0x21c,'fadd','s0, s8, s0')]: req(*x)
 with a.memory_image.open('rb') as f:
  f.seek(0xd2ac8c);pct=f.read(4);f.seek(0xd2b6d0);rift=f.read(4)
 if pct.hex()!='0ad7233c' or rift.hex()!='17b7d138': raise ValueError('static literals changed')
 out = {
  'schemaVersion': 1,
  'contractType': 'original-native-hunter-outgoing-helpers-pass8',
  'runtimeCompatibility': 'evidence-only-disconnected',
  'sourceSha256': hashlib.sha256(a.source.read_bytes()).hexdigest(),
  'literals': {
   'percent': {'offset':'0xD2AC8C','raw':pct.hex(),'float32':struct.unpack('<f',pct)[0]},
   'riftScale': {'offset':'0xD2B6D0','raw':rift.hex(),'float32':struct.unpack('<f',rift)[0]},
  },
  'methods': {
   'getSlayerDamage': {'moduleOffset':sc['moduleOffset'],'size':len(sb),'sha256':hashlib.sha256(sb).hexdigest()},
   'getRiftNpcBuffDamage': {'moduleOffset':rc['moduleOffset'],'size':len(rb),'sha256':hashlib.sha256(rb).hexdigest()},
  },
  'slayer': {
   'commonNamedInput': 'positive RidingPetSlayerDemUp@StatusData+0x74C is added before race branches',
   'raceSelector': 'DataManager.mAdminEvilData[input].race@+0x100',
   'raceBranches': {
    '1': {'gearPropertyIndex':11,'namedAdds':['CollectionPrimateDem@0x574','RelicCollectionPrimateDem@0x5C8']},
    '2': {'gearPropertyIndex':13,'namedAdds':['CollectionUndeadDem@0x57C','RelicCollectionUndeadDem@0x5D0']},
    '3': {'gearPropertyIndex':12,'namedAdds':['CollectionEvilDem@0x578','RelicCollectionEvilDem@0x5CC']},
    '4': {'gearPropertyIndex':46,'namedAdds':['CollectionAnimalDem@0x580','RelicCollectionAnimalDem@0x5D4']},
    '5': {'gearPropertyIndex':45,'namedAdds':['CollectionBossDem@0x570','RelicCollectionBossDem@0x5C4']},
   },
   'gearPropertyEquation': 'accumulator += (element0-element1)*0.01 where the branch-specific array exists',
   'opaqueExtras': ['job-trait 21 gates and AdminJobTraitData fields','UserData+0xB78/+0xB80 decoded integer contribution','helper at 0x2FA2D94 and its field +0xD4'],
   'return': 'ObscuredFloat accumulator',
  },
  'rift': {
   'initial':'0.0',
   'inputGate':'decoded input equals either GameManager static int at +0xC10 or +0xC30',
   'lookupGate':'UserData+0xCF8 exists, contains required keys, and nested value exists',
   'equation':'result += decoded nested integer * 0.0001f',
   'unresolved':['names/types of UserData+0xCF8 nested dictionaries','meanings of GameManager static ints +0xC10/+0xC30 and lookup keys'],
  },
  'unresolved':['semantic meaning of method ObscuredInt input','exact semantics of race enum values despite named stat columns','opaque Slayer extras and their ordering labels'],
 }
 a.output.write_text(json.dumps(out,indent=2)+'\n')
if __name__=='__main__':main()
