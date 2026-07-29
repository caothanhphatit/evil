import { describe, expect, it } from "vitest";
import { formatLevelCosts, projectBuildingEvidence } from "./building-evidence";
import type { EvidenceBuildingRegistry } from "./building-registry";

describe("building evidence projection", () => {
  it("uses recovered localization and exact per-level costs", () => {
    const view = projectBuildingEvidence(fixture(), "build_10", "vi-VN");
    expect(view).toMatchObject({
      id: "build_10",
      name: "Lò Rèn",
      maxLevel: 2,
      maxBuild: 1,
      gridSize: [2, 2],
      spriteAssetId: "build_10",
    });
    expect(view?.levels[1].requiredTownHallLevel).toBe(5);
    expect(formatLevelCosts(view!, 2)).toBe("Vàng 4.800 · Mảnh Cây Chi Hông 10");
  });

  it("never treats the bounty RequestPop as a Trading Post request UI", () => {
    const registry = fixture();
    const building = registry.buildings.rows[0];
    building.buildId = field("any-decoded-id");
    building.visualBinding = { popupClass: field("RequestPop"), spriteAssetId: field("build_10") };
    expect(projectBuildingEvidence(registry, "any-decoded-id")?.popupRoute).toBeNull();
  });

  it("routes purchase reservations only through the decoded BuildingPop capability", () => {
    const registry = fixture();
    const building = registry.buildings.rows[0];
    building.buildId = field("decoded-trader");
    building.visualBinding = { popupClass: field("BuildingPop"), spriteAssetId: field("build_3") };
    building.capabilityIds = collection([{ key: "cap-ref", id: field("capability:purchase") }]);
    registry.catalogs.capabilities = collection([{
      key: "purchase",
      capabilityId: field("capability:purchase"),
      kind: field("loot-purchase-reservations"),
      parameters: field({ description: { en: "Purchases loot" } }),
    }]);
    expect(projectBuildingEvidence(registry, "decoded-trader")?.popupRoute).toBe("request");
  });

  it("uses decoded capability descriptions but does not infer a popup call-site", () => {
    const registry = fixture();
    const building = registry.buildings.rows[0];
    building.buildId = field("decoded-trader");
    building.capabilityIds = collection([{ key: "cap-ref", id: field("capability:purchase") }]);
    registry.catalogs.capabilities = collection([{
      key: "purchase",
      capabilityId: field("capability:purchase"),
      kind: field("loot-purchase-reservations"),
      parameters: field({ description: { en: "Purchases loot", vi: "Mua chiến lợi phẩm" } }),
    }]);
    expect(projectBuildingEvidence(registry, "decoded-trader", "vi")?.popupRoute).toBeNull();
    expect(projectBuildingEvidence(registry, "decoded-trader", "vi")?.description).toBe("Mua chiến lợi phẩm");
  });

  it("fails closed when popup and capabilities are unresolved", () => {
    const view = projectBuildingEvidence(fixture(), "build_10");
    expect(view?.popupRoute).toBeNull();
    expect(view?.actionBlockedReason).toMatch(/controller dispatch/);
  });

  it("routes the confirmed Enhancement Forge build_15 to its blocker shell", () => {
    const registry = fixture();
    registry.buildings.rows[0].buildId = field("build_15");
    const view = projectBuildingEvidence(registry, "build_15");
    expect(view?.popupRoute).toBe("gear-enhancement");
    expect(view?.actionBlockedReason).toBe("popup-template-binding");
  });
});

function fixture(): EvidenceBuildingRegistry {
  const unresolved = (reason: string) => ({ state: "unresolved", confidence: "unknown", value: null, evidence: [], requiredEvidence: reason });
  const collection = (rows: Array<Record<string, unknown>>, resolved = true) => ({
    binding: resolved ? binding() : unresolved("Decode the building controller dispatch."),
    rows,
  });
  const amount = (key: string, item: string, quantity: number) => ({ key, itemId: field(item), quantity: field(quantity) });
  const level = (number: number, costs: Array<Record<string, unknown>>, townHallLevel: number) => ({
    key: `level-${number}`,
    level: field(number),
    upgradeCosts: collection(costs),
    conditions: collection([{
      key: "town-hall-level",
      subjectId: field("build_1.level"),
      operator: field("greater-than-or-equal"),
      operand: field(townHallLevel),
    }]),
  });
  return {
    schemaVersion: 1,
    contractType: "building-registry",
    registryId: "evil-hunter-1.411.buildings-v1",
    runtimeState: "blocked",
    legacy: { game: "Evil Hunter Tycoon", version: "1.411", package: "com.superplanet.evilhunter" },
    evidencePolicy: {
      semanticFields: "evidence-required-per-field",
      unresolvedValues: "fail-closed-null-or-empty",
      visualBinding: "separate-from-gameplay-semantics",
    },
    evidenceSources: [],
    catalogs: {
      items: collection([{ key: "material-11", itemId: field("material:11"), displayName: field({ en: "Heartwood Fragment", vi: "Mảnh Cây Chi Hông" }) }]),
      products: collection([]),
      capabilities: collection([]),
    },
    buildings: collection([{
      key: "build_10",
      buildId: field("build_10"),
      displayName: field({ en: "Blacksmith", vi: "Lò Rèn" }),
      levels: collection([
        level(1, [amount("gold", "currency:gold", 660)], 2),
        level(2, [amount("gold", "currency:gold", 4800), amount("material", "material:11", 10)], 5),
      ]),
      buildRows: collection([]),
      sourceData: { maxBuild: field(1), gridSize: field([2, 2]) },
      capabilityIds: collection([], false),
      visualBinding: {
        popupClass: unresolved("Decode the building controller dispatch."),
        spriteAssetId: field("build_10"),
        townPosition: unresolved("Decode placement."),
      },
    }]),
    releaseGate: { runnable: false, blockingPaths: ["buildings.rows[0].capabilityIds.binding"], reason: "Decode incomplete" },
  } as unknown as EvidenceBuildingRegistry;
}

function binding(): { state: "resolved"; confidence: "confirmed"; evidence: []; requiredEvidence: null } {
  return { state: "resolved", confidence: "confirmed", evidence: [], requiredEvidence: null };
}

function collection(rows: Array<Record<string, unknown>>) {
  return { binding: binding(), rows };
}

function field(value: unknown) {
  return { ...binding(), value };
}
