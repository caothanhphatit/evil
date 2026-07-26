import { describe, expect, it } from "vitest";
import { validateAtomicActorBundles, validateVisibleWorldRelease, verifyBytes, type VisibleWorldRelease } from "./visible-world-release";

describe("visible-world runtime trust boundary", () => {
  it("accepts schema v3 only when fixture diagnostics and atomic actor paths are explicit", () => {
    expect(validateVisibleWorldRelease(releaseFixture()).runtimeDiagnostics.unresolved).toContain("starter-skins");
    const mixed = releaseFixture();
    mixed.actors[0].atlas.publicPath = "/content/releases/visible-world-v1/actors/Npc/hunter.atlas";
    expect(() => validateAtomicActorBundles(mixed)).toThrow(/not atomic/);
  });

  it("rejects stale or modified bytes", async () => {
    const payload = new TextEncoder().encode("verified");
    await expect(verifyBytes("asset", payload, 8, "1c34f88707b55e6104c4eb20e71ffa3d33e414b71ef689a15fad0640d0ac58cb")).resolves.toBeUndefined();
    await expect(verifyBytes("asset", new TextEncoder().encode("modified"), 8, "1c34f88707b55e6104c4eb20e71ffa3d33e414b71ef689a15fad0640d0ac58cb")).rejects.toThrow(/checksum mismatch/);
  });
});

function releaseFixture(): VisibleWorldRelease {
  const asset = (path: string) => ({ publicPath: path, bytes: 1, sha256: "0".repeat(64) });
  const families = ["hunter", "Chief", "Npc", "npc_animal", "pet", "mon_goldblin", "mon_a_01_1"];
  return {
    schemaVersion: 3,
    releaseId: "visible-world-v1",
    releaseState: "development-evidence",
    bindingState: "mixed-resolved-and-unresolved",
    evidencePolicy: { runtimeAuthority: "presentation-only", fixtureLabelRequired: true },
    runtimeDiagnostics: { fixture: true, unresolved: ["starter-skins"] },
    map: { ...asset("/content/releases/visible-world-v1/maps/map_new01.png"), runtimeUse: "migration-fixture", evidence: { note: "candidate" } },
    fieldMap: asset("/content/releases/visible-world-v1/maps/map_new01.png"),
    village: { bindingState: "partial-scene-derived", completeness: "partial", tiles: [], foreground: [], decorations: [] },
    actors: families.map((family) => ({
      family,
      runtimeUse: "migration-fixture" as const,
      evidence: { note: "unresolved skin and spawn" },
      skeleton: asset(`/content/releases/visible-world-v1/actors/${family}/${family}.json`),
      atlas: asset(`/content/releases/visible-world-v1/actors/${family}/${family}.atlas`),
      texture: asset(`/content/releases/visible-world-v1/actors/${family}/${family}.png`),
    })),
  };
}
