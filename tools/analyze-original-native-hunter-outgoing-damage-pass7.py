#!/usr/bin/env python3
"""Verify and normalize exact getDamage/getCriticalDamage SSA anchors."""
from __future__ import annotations
import argparse, hashlib, json, struct
from pathlib import Path
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json'
IMAGE=Path('/tmp/evil-libil2cpp-memory.bin')
OUT=ROOT/'reverse-engineering/evidence/original-native-hunter-outgoing-damage-pass7.json'

def get_method(payload,name): return next(m for m in payload['methods'] if m['className']=='HunterCtrl' and m['methodName']==name)
def code(method):
 c=method['candidates'][0]; return c,bytes.fromhex(c['codeHex'])
def insns(c,b): return {i.address-int(c['moduleOffset'],16):(i.mnemonic,i.op_str) for i in Cs(CS_ARCH_ARM64,CS_MODE_ARM).disasm(b,int(c['moduleOffset'],16))}
def require(ins,off,mnemonic,operand):
 got=ins.get(off)
 if got!=(mnemonic,operand): raise ValueError(f'0x{off:x}: expected {(mnemonic,operand)}, got {got}')

def main():
 p=argparse.ArgumentParser(); p.add_argument('--source',type=Path,default=SOURCE); p.add_argument('--memory-image',type=Path,default=IMAGE); p.add_argument('--output',type=Path,default=OUT); a=p.parse_args()
 payload=json.loads(a.source.read_text())['record']['payload']; gd_c,gd=code(get_method(payload,'getDamage')); cd_c,cd=code(get_method(payload,'getCriticalDamage')); gi,ci=insns(gd_c,gd),insns(cd_c,cd)
 for x in [(gi,0x2474,'fmul','d0, d10, d0'),(gi,0x2478,'fadd','d0, d10, d0'),(gi,0x247c,'fmul','d1, d0, d1'),(gi,0x2480,'fadd','d0, d0, d1'),(gi,0x2484,'fmul','d1, d0, d14'),(gi,0x2488,'fadd','d0, d0, d1'),(gi,0x2490,'fmul','d0, d0, d11'),(gi,0x2494,'fmul','d0, d0, d1'),(gi,0x24a0,'fmul','d0, d0, d1'),(gi,0x24a8,'fmul','d0, d0, d1'),(gi,0x24b0,'fmul','d0, d0, d1'),(gi,0x24bc,'fcvtzs','x9, d0')]: require(*x)
 for x in [(ci,0xa4,'fmov','s9, #1.75000000'),(ci,0xf8,'fmul','s0, s0, s1'),(ci,0x100,'fadd','s9, s0, s1'),(ci,0x130,'fmul','s0, s0, s1'),(ci,0x134,'fadd','s9, s9, s0'),(ci,0x164,'fmul','s0, s0, s1'),(ci,0x168,'fadd','s9, s9, s0'),(ci,0x194,'fadd','s9, s9, s0'),(ci,0x1f8,'fadd','s9, s9, s0'),(ci,0x274,'fadd','s9, s9, s0'),(ci,0x2d8,'fadd','s9, s9, s0'),(ci,0x320,'fadd','s9, s9, s0'),(ci,0x368,'fadd','s9, s9, s0'),(ci,0x3b0,'fadd','s9, s9, s0'),(ci,0x514,'fmul','s0, s0, s1'),(ci,0x518,'fadd','s0, s8, s0'),(ci,0x66c,'fadd','s0, s8, s0'),(ci,0x730,'fadd','s0, s9, s0')]: require(*x)
 with a.memory_image.open('rb') as f: f.seek(0xd2abac); cap_raw=f.read(4)
 cap=struct.unpack('<f',cap_raw)[0]
 if cap_raw.hex()!='6666e63f': raise ValueError('critical cap literal changed')
 result={
  'schemaVersion':1,'contractType':'original-native-hunter-outgoing-damage-pass7','runtimeCompatibility':'evidence-only-disconnected',
  'source':{'path':a.source.name,'sha256':hashlib.sha256(a.source.read_bytes()).hexdigest()},
  'methods':{'getDamage':{'moduleOffset':gd_c['moduleOffset'],'sha256':hashlib.sha256(gd).hexdigest()},'getCriticalDamage':{'moduleOffset':cd_c['moduleOffset'],'sha256':hashlib.sha256(cd).hexdigest()}},
  'getDamageFinalSsa':[
   'A = D10 * (1 + float64(S12))','B = A * (1 + float64(S13))','C = B * (1 + D14)','D = C * D11','E = D * float64(S15)','F = E * float64(stackFloat@0xC)','G = F * float64(S8)','H = G * float64(S9)','result = truncTowardZero(H)'],
  'getCriticalDamageSsa':{
   'initial':'C0 = 1.75 + positive(UserData+0xA14)*0.01',
   'namedAdds':['positive(CollectionCriDem@0x558)*0.01','positive(RelicCollectionCriDem@0x5AC)*0.01','positive(VillagePetCriDemUp@0x548)','positive(RidingPetCriDemUp@0x658)','gated positive(SylphBlessCriDemUp@0x878)','positive(HeroicJobTraitCriDemUp@0x61C)'],
   'opaqueHunterAdds':['positive(BDDEONCMGHK@0x7FC)','positive(FBNMALOOBKK@0x810)','positive(AKBENLLFPCC@0x854)'],
   'gearPropertyTemporary':[
    'GearProperty[43]: when element0>0 OR element1>=1, T = current + (element0-element1)*0.01',
    'GearProperty[59]: when element0>=1 AND DataManager.mAdminEvilData[input].race==1, T = current + (element0-element1)*0.01',
    'GearProperty[14]: when element0>=1 AND T>1.8f, T=1.8f'],
   'return':'criticalFactor = C + T','capLiteral':{'moduleOffset':'0xD2ABAC','rawHex':cap_raw.hex(),'float32':cap}},
  'gates':{'sylph':'HunterCtrl.KPJLBPKPCCG@0x5E4','gearProperty':'StatusData.GearProperty@0x210','raceArray':'DataManager.mAdminEvilData@0x40; AdminEvilData.race@0x100','input':'method ObscuredInt argument; semantic meaning unresolved'},
  'unresolved':['semantic name/writers of UserData+0xA14','writers/meaning of three opaque HunterCtrl float fields','producer meanings of getDamage D10/S12/S13/D14/D11/S15/stack+0xC/S8/S9','skill coefficient boundary and complete caller vectors','monster armor/minimum-damage consumer']}
 a.output.write_text(json.dumps(result,indent=2)+'\n')
if __name__=='__main__': main()
