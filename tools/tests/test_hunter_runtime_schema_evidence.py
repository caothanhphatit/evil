from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "reverse-engineering/evidence"


def load(name: str) -> dict[str, object]:
    return json.loads((EVIDENCE / name).read_text())


def classes(document: dict[str, object]) -> dict[str, dict[str, object]]:
    payload = document["record"]["payload"]
    return {item["name"]: item for item in payload["classes"]}


def fields(record: dict[str, object]) -> dict[str, dict[str, object]]:
    return {item["name"]: item for item in record["fields"]}


class HunterRuntimeSchemaEvidenceTest(unittest.TestCase):
    def setUp(self) -> None:
        self.documents = [
            load("hunter-info-runtime-schema-android-api30-v1.json"),
            load("hunter-domain-runtime-schema-android-api30-v1.json"),
            load("hunter-manager-runtime-schema-android-api30-v1.json"),
            load("hunter-collection-runtime-schema-android-api30-v1.json"),
        ]

    def test_capture_metadata_and_target_coverage(self) -> None:
        for document in self.documents:
            capture = document["capture"]
            payload = document["record"]["payload"]
            self.assertEqual(capture["packageId"], "com.superplanet.evilhunter")
            self.assertEqual(capture["versionName"], "1.411")
            self.assertEqual(capture["versionCode"], "26071501")
            self.assertEqual(capture["deviceAbi"], "arm64-v8a")
            self.assertEqual(capture["fridaClientVersion"], capture["fridaServerVersion"])
            self.assertEqual(document["record"]["kind"], "hunter-info-schema")
            self.assertEqual(payload["missing"], [])

    def test_primary_hunter_and_save_shapes(self) -> None:
        runtime = classes(self.documents[0])
        save = fields(runtime["SaveData"])
        self.assertEqual(
            {name: save[name]["type"] for name in ("<index>k__BackingField", "<data>k__BackingField")},
            {
                "<index>k__BackingField": "System.String",
                "<data>k__BackingField": "System.String",
            },
        )
        self.assertEqual(save["<action>k__BackingField"]["type"], "System.Boolean")
        self.assertEqual(save["<clear>k__BackingField"]["type"], "System.Boolean")

        hunter = fields(runtime["HunterData"])
        self.assertEqual(
            hunter["<gearInventory>k__BackingField"]["type"],
            "System.Collections.Generic.Dictionary<System.String,GearData>",
        )
        self.assertEqual(
            hunter["<skill>k__BackingField"]["type"],
            "System.Collections.Generic.Dictionary<System.String,SkillData>",
        )
        self.assertEqual(
            hunter["<GUP_Property_LV>k__BackingField"]["type"],
            "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        )

    def test_api35_primary_schema_matches_api30(self) -> None:
        api30 = self.documents[0]
        api35 = load("hunter-info-runtime-schema-android-api35-v1.json")

        self.assertEqual(api35["capture"]["androidApi"], "35")
        self.assertEqual(api35["capture"]["androidRelease"], "15")
        self.assertEqual(api35["record"]["payload"]["missing"], [])
        self.assertEqual(
            api35["record"]["payload"]["classes"],
            api30["record"]["payload"]["classes"],
        )

    def test_nested_owned_data_shapes(self) -> None:
        runtime = classes(self.documents[1])
        skill = fields(runtime["SkillData"])
        self.assertEqual(skill["<skillIndex>k__BackingField"]["type"], "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt")
        self.assertEqual(skill["<coolTime>k__BackingField"]["type"], "CodeStage.AntiCheat.ObscuredTypes.ObscuredDouble")

        riding_pet = fields(runtime["RidingPetData"])
        self.assertEqual(
            riding_pet["<PetGearInventory>k__BackingField"]["type"],
            "System.Collections.Generic.Dictionary<System.String,RidingPetGearData>",
        )

    def test_manager_and_collection_boundaries(self) -> None:
        manager_runtime = classes(self.documents[2])
        game_manager = fields(manager_runtime["GameManager"])
        self.assertEqual(game_manager["mHunterData"]["type"], "HunterDataDic")
        self.assertEqual(game_manager["mHunterWaitData"]["type"], "HunterDataDic")
        self.assertEqual(game_manager["mSaveData"]["type"], "SaveDataDic")
        self.assertEqual(game_manager["hunterSaveData"]["type"], "System.String")

        collection_runtime = classes(self.documents[3])
        hunter_data = fields(collection_runtime["HunterDataDic"])
        self.assertEqual(
            hunter_data["<data>k__BackingField"]["type"],
            "System.Collections.Generic.Dictionary<System.String,HunterData>",
        )
        self.assertEqual(
            hunter_data["<ridingPetData>k__BackingField"]["type"],
            "System.Collections.Generic.Dictionary<System.String,RidingPetData>",
        )


if __name__ == "__main__":
    unittest.main()
