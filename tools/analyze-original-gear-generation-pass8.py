#!/usr/bin/env python3
"""Normalize the exact GearData formula/generation boundary without enum guesses."""
import json, hashlib
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/'reverse-engineering/evidence/original-native-gear-formula-analysis-v1.json'
SCHEMA=ROOT/'reverse-engineering/evidence/admin-gear-formula-runtime-schema-api35-v1.json'
OUT=ROOT/'reverse-engineering/evidence/original-gear-generation-boundary-v8.json'

def ref(p):
 b=p.read_bytes(); return {'path':p.relative_to(ROOT).as_posix(),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}

def build():
 d=json.loads(SRC.read_text()); s=json.loads(SCHEMA.read_text())
 fields=[x['semanticName'] for x in d['gearDataInput']['fields']]
 if fields!=['index','gearIndex','quality','level','rating']: raise ValueError('GearData direct inputs changed')
 opts={x['name']:x['offset'] for x in s['fields']}
 if opts!={'ratingValue':112,'firstValue':128,'firstPercent':152,'secondValue':160,'plusType':192,'plusValue':200,'minusType':208,'minusValue':216}: raise ValueError('AdminGearData schema changed')
 return {'schemaVersion':8,'contractType':'original-gear-generation-boundary-evidence','runtimeCompatibility':'evidence-only','sources':[ref(SRC),ref(SCHEMA)],
 'confirmedFormulaInputs':fields,
 'confirmedFormulas':{'armorAndAcc':d['gearArmorAndAcc']['neutralExpression'],'damage':d['gearDamage']['expression'],'ratingSelection':d['gearDamage']['ratingSelection'],'qualityMultipliers':d['gearArmorAndAcc']['qualityMultipliers'],'rounding':'round to nearest, ties to even'},
 'adminOptionSchema':opts,
 'directReaderBoundary':{'GetGearDamageArmorAccReadPlusMinusOrRunes':False,'reason':'The three exact bodies read only index, gearIndex, quality, level and rating from GearData.'},
 'unresolved':{'levelAdjustmentWriter':None,'plusTypeEnumMeanings':None,'minusTypeEnumMeanings':None,'plusMinusRollOrder':None,'enhancementWriterAndOrder':None,'runeParticipationAndOrder':None,'generationQualityRatingOrder':None},
 'implementationBoundary':{'liveIntegrationAllowed':False,'requiredNextEvidence':'Exact GearData creation/enhancement/rune writer bodies and the caller supplying the level adjustment before GetFirstPercent.'}}

if __name__=='__main__': OUT.write_text(json.dumps(build(),ensure_ascii=True,indent=2)+'\n'); print(f'Wrote {OUT}')
