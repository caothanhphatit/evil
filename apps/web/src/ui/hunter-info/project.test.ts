import { describe, expect, it } from "vitest";
import type { HunterView } from "../hunter-roster";
import { projectHunterInfo } from "./project";

const hunter: HunterView = {
  id: "hunter-1", numericId: 1, name: "Ocos", rosterState: "active", queuePosition: null,
  level: 24, xp: 262, classId: "h1", className: "Berserker", classFamily: "H1",
  rarityId: "rare", rarityName: "RARE", traitName: "Job Trait", traits: [], action: "Fun",
  animation: null, hp: 775, maxHp: 7283, stamina: 96, maxStamina: 100,
  satiety: 100, maxSatiety: 140, mood: 57, maxMood: 120, attack: 639, defense: 444,
  gold: 6684, portrait: "/content/hunter.png",
  skills: [{ id: "legacy", name: "Legacy", level: 1, icon: null, ready: true }],
  hunt: null,
};

describe("projectHunterInfo", () => {
  it("projects explicit nested Hunter information without confusing Characteristic and Job Trait", () => {
    const view = projectHunterInfo({
      hunter_info: {
        characteristic_name: "Charismatic",
        status: { dps_milli: 245770, critical_rate_bps: 700, attack_speed_milli: 2600, evasion_rate_bps: 300, awakening: { current: 0, maximum: 4 } },
        equipment_slots: [
          { slot_id: "weapon", catalog_kind: "weapon", catalog_index: 0, display_name: "Junk Sword", icon_path: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png", presentation_gender: "female", required_class_id: "h1", locked: false, evidence_state: "web_rebuild_test_fixture" },
          { slot_id: "armor", icon_path: "/content/releases/evil-hunter-1.411/gear-icons/armor-0.png", locked: false },
        ],
        skills: [{ skill_id: "fury", display_name: "Fury", level: 1, group: "Basic Skill", description: "Attack quickly." }],
        growth: { secret_points: 0, nodes: [{ node_id: "node-1", icon_path: "/content/node.png", points: 0, max_points: 100, order: 1 }] },
        riding_pet: { mounted: false, can_move_to_ranch: true },
        materials: [{ material_id: "wood", icon_path: "/content/wood.png", quantity: 18, order: 1 }],
      },
    }, hunter);
    expect(view.title).toBe("Charismatic Ocos");
    expect(view.dps).toBe(245.77);
    expect(view.equipment).toHaveLength(8);
    expect(view.equipment[5]).toMatchObject({ id: "weapon", catalogKind: "weapon", catalogIndex: 0, name: "Junk Sword", presentationGender: "female", requiredClassId: "h1", evidenceState: "web_rebuild_test_fixture", icon: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png" });
    expect(view.equipment[6]).toMatchObject({ id: "armor", icon: "/content/releases/evil-hunter-1.411/gear-icons/armor-0.png" });
    expect(view.skills?.[0]).toMatchObject({ id: "fury", group: "Basic Skill", description: "Attack quickly." });
    expect(view.growth?.nodes[0]).toMatchObject({ id: "node-1", maxPoints: 100 });
    expect(view.riding).toEqual({ mounted: false, canMoveToRanch: true });
    expect(view.materials?.[0]).toMatchObject({ id: "wood", quantity: 18 });
  });

  it("does not borrow roster skills, Job Trait, or pooled materials when nested payloads are absent", () => {
    const view = projectHunterInfo({}, hunter);
    expect(view.title).toBe("Ocos");
    expect(view.skills).toBeNull();
    expect(view.growth).toBeNull();
    expect(view.riding).toBeNull();
    expect(view.materials).toBeNull();
  });

  it("drops material rows that do not have explicit per-Hunter identity, icon and quantity", () => {
    const view = projectHunterInfo({ hunter_info: { materials: [{ material_id: "wood", quantity: 4 }, { icon_path: "/content/ore.png", quantity: 3 }] } }, hunter);
    expect(view.materials).toEqual([]);
  });

  it("projects the authoritative equipped rebuild weapon into the Detail weapon slot", () => {
    const instanceId = "00000000-0000-4000-8000-000000000123";
    const view = projectHunterInfo({
      hunter_info: {
        equipment_slots: [{ slot_id: "weapon", display_name: "Legacy Sword", icon_path: "/content/legacy.png" }],
        weapons: [{
          gear_instance_id: instanceId,
          product_id: "recipe:weapon:0:rating:0",
          weapon_id: "wp_berserker_000",
          display_name_en: "Chipped Iron Greatsword",
          display_name_vi: "Đại Kiếm Sắt Mẻ",
          icon_path: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png",
          quality: 3,
          attack_damage: 88,
          attack_damage_min: 60,
          attack_damage_max: 96,
          enhancement_level: 0,
          compatible: true,
          equipped: true,
          ruleset: "web-rebuild-weapon-core-v1",
        }],
      },
    }, hunter);

    expect(view.equipment[5]).toMatchObject({
      id: "weapon",
      catalogKind: `rebuild_weapon_instance:${instanceId}`,
      name: "Đại Kiếm Sắt Mẻ",
      icon: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png",
      evidenceState: "web-rebuild-weapon-core-v1",
    });
  });
});
