import { describe, expect, it } from "vitest";
import { hunterActorVisual, hunterClassTone, hunterPercent, hunterRarityPresentation, hunterWorldEntityId, projectHunterRoster } from "./hunter-roster";
import { hunterPaperDollVisual } from "../game/hunter-actor-presentation";
import { hunterWeaponAttachment } from "../game/hunter-spine-presentation";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const repositoryRoot = resolve(process.cwd(), "../..");

describe("projectHunterRoster", () => {
  it("projects active slots, waiting order, class, traits, skills and action data", () => {
    const view = projectHunterRoster({
      hunter_roster: {
        active_capacity: 8,
        active_hunters: [{ hunter_id: 7, display_name: "Rin", level: 12, class_id: "berserker", class_name: "Berserker", class_family: "h1", trait_name: "Swift", traits: [], current_hp: 75, max_hp: 100, attack: 22, action_state: "farming", animation: "hunter_walk", skills: [{ skill_id: "slash", display_name: "Slash", level: 2 }] }],
        waiting_hunters: [{ hunter_id: 9, display_name: "Mina" }],
      },
      world: { entities: [] },
    }, null);
    expect(view.capacity).toBe(8);
    expect(view.active[0]).toMatchObject({ id: "hunter-7", name: "Rin", classFamily: "H1", traitName: "Swift", action: "farming", hp: 75, attack: 22 });
    expect(view.active[0].skills[0]).toMatchObject({ id: "slash", name: "Slash", level: 2 });
    expect(view.waiting[0]).toMatchObject({ id: "hunter-9", queuePosition: 1 });
    expect(view.selectedId).toBe("hunter-7");
  });

  it("fails closed instead of deriving a roster from legacy world entities", () => {
    const view = projectHunterRoster({
      hunter_roster: { infirmary: { hunters: [] }, product_services: [] },
      world: { entities: [{ descriptor: { entity_id: "hunter-3", kind: "hunter" }, animation: "hunter_stay" }] },
    });
    expect(view.capacity).toBe(8);
    expect(view.active).toEqual([]);
    expect(view.resolved).toBe(false);
  });

  it("reports an invalid server projection that exceeds town capacity", () => {
    const active_hunters = Array.from({ length: 9 }, (_, index) => ({ hunter_id: index + 1, display_name: `Hunter ${index + 1}` }));
    expect(projectHunterRoster({ hunter_roster: { active_capacity: 8, active_hunters }, world: {} }).constraintViolation).toBe("Vượt sức chứa thị trấn: 9/8");
  });

  it("projects authoritative hunt progress and loot without calculating outcomes", () => {
    const view = projectHunterRoster({
      hunter_roster: { active_hunters: [{ hunter_id: 2, display_name: "Hunter 2", hunt: { status: "returning", zone_id: "migration-zone-1", progress_ticks: 10, required_ticks: 10, loot: [{ item_id: "young_lycan_fur", quantity: 1 }] } }] },
      world: { entities: [] },
    });
    expect(view.active[0].hunt).toEqual({ status: "returning", zoneId: "migration-zone-1", progressTicks: 10, requiredTicks: 10, loot: [{ itemId: "young_lycan_fur", quantity: 1 }] });
  });
});

describe("hunter helpers", () => {
  it("locate closes the roster and reuses normal world Hunter selection", async () => {
    const source = await readFile(resolve(import.meta.dirname, "../app/hunter-controller.ts"), "utf8");
    expect(source).toContain("setHunterRosterVisibility(false);");
    expect(source).toContain("hunterWorldCommandMenu.selectHunter({");
    expect(source).toContain("world?.focusEntity(worldEntityId)");
  });

  it("opens Hunter Info from both the avatar and the explicit Info control", async () => {
    const source = await readFile(resolve(import.meta.dirname, "../app/hunter-controller.ts"), "utf8");
    expect(source).toContain('const avatar = document.createElement("button")');
    expect(source).toContain('avatar.setAttribute("aria-label", t("roster.info_aria", { name: hunter.name }))');
    expect(source).toContain('avatar.addEventListener("click", () => openHunterInfo(avatar))');
    expect(source).toContain('info.addEventListener("click", () => openHunterInfo(info))');
  });

  it("resolves a roster Hunter to its selectable world actor for locate", () => {
    const roster = projectHunterRoster({ hunter_roster: { active_hunters: [{ hunter_id: 3, display_name: "Hunter 3" }] }, world: { entities: [] } });
    expect(hunterWorldEntityId({ world: { entities: [{ descriptor: { entity_id: "village-hunter-3", source_skeleton_name: "hunter" }, selectable: true }] } }, roster.active[0])).toBe("village-hunter-3");
  });
  it("clamps gauges and resolves class-family Spine composition", () => {
    expect(hunterPercent(150, 100)).toBe(100);
    expect(hunterPercent(null, 100)).toBeNull();
    expect(hunterActorVisual({ hunter_id: 1, class_family: "h4", hunter_visual: { weapon_skin: "weapon_h4_a_01" } })).toEqual({
      skinNames: ["hunter_m_01", "costum_h4_01", "weapon_h4_a_01"],
      animation: null,
      tint: 0xffffff,
      signature: "hunter_m_01|costum_h4_01|weapon_h4_a_01:ffffff",
    });
  });

  it("maps the server skill presentation marker to the recovered class hit clip", () => {
    expect(hunterActorVisual({ hunter_id: 1, class_family: "H3", animation: "h3_skill" }).animation).toBe("h3_hit");
  });

  it("maps the five authoritative rarity names to source-style roster letters", () => {
    expect(["normal", "rare", "superior", "heroic", "legendary"].map((rarity) => hunterRarityPresentation(rarity, null))).toEqual([
      { key: "normal", letter: "N" },
      { key: "rare", letter: "R" },
      { key: "superior", letter: "S" },
      { key: "heroic", letter: "H" },
      { key: "legendary", letter: "L" },
    ]);
    expect(hunterRarityPresentation("unknown", "Unresolved")).toBeNull();
    expect(hunterClassTone("H4")).toBe("h4");
    expect(hunterClassTone(null)).toBe("unresolved");
  });

  it("gives the eight demo Hunters distinct confirmed aggregate compositions", () => {
    const visuals = Array.from({ length: 8 }, (_, index) => hunterActorVisual({ descriptor: { entity_id: `hunter-${index + 1}` }, class_family: "H1", animation: "hunter_stay" }));
    expect(new Set(visuals.map((visual) => visual.skinNames[0])).size).toBe(2);
    expect(visuals[0].skinNames).toEqual(["hunter_m_01", "costum_h1_01", "weapon_h1_a_01"]);
    expect(visuals[1].skinNames).toEqual(["hunter_f_01", "costum_h1_02", "weapon_h1_a_01"]);
    expect(new Set(visuals.map((visual) => visual.signature)).size).toBe(8);
  });

  it("uses the gendered first outfit for every known job family", () => {
    expect(["H1", "H2", "H3", "H4", "H5"].map((family) => hunterActorVisual({ hunter_id: 1, class_family: family }).skinNames)).toEqual([
      ["hunter_m_01", "costum_h1_01", "weapon_h1_a_01"],
      ["hunter_m_01", "costum_h2_01", "weapon_h2_a_01"],
      ["hunter_m_01", "costum_h3_01", "weapon_h3_a_01"],
      ["hunter_m_01", "costum_h4_01", "weapon_h4_a_01"],
      ["hunter_m_01", "costum_h5_01", "weapon_h5_a_01"],
    ]);
    expect(["H1", "H2", "H3", "H4", "H5"].map(hunterWeaponAttachment)).toEqual([
      { slot: "weapon_01", attachment: "sword" },
      { slot: "weapon_02", attachment: "hammer" },
      { slot: "weapon_03", attachment: "bow" },
      { slot: "weapon_04", attachment: "wand" },
      { slot: "weapon_05", attachment: "spear" },
    ]);
  });

  it("uses the packaged full-outfit class composition for list and Detail paper dolls", () => {
    expect(["H1", "H2", "H3", "H4", "H5"].map((family) => hunterPaperDollVisual({ hunter_id: 1, class_family: family }).skinNames)).toEqual([
      ["All_h1"],
      ["All_h2"],
      ["All_h3"],
      ["All_h4"],
      ["All_h5"],
    ]);
  });

  it("removes only non-paper-doll effect attachments before fitting the actor", async () => {
    const presentation = await readFile(resolve(repositoryRoot, "apps/web/src/game/hunter-spine-presentation.ts"), "utf8");
    expect(presentation).toContain('name.startsWith("effect") || name.endsWith("_effect")');
    expect(presentation).toContain("slot.setAttachment(null)");
  });
});
