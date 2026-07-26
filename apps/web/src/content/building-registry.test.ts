import { describe, expect, it, vi } from "vitest";
import { loadVerifiedBuildingEvidenceRegistry, loadVerifiedBuildingRegistry, validateRuntimeBuildingRegistry, verifyRegistryBytes } from "./building-registry";

const SOURCE_ID = "decoded-source";

describe("building registry runtime trust boundary", () => {
  it("rejects a registry while its release gate is blocked", () => {
    const registry = runtimeReadyFixture();
    registry.runtimeState = "blocked";
    registry.releaseGate = { runnable: false, blockingPaths: ["buildings.binding"], reason: "Decode is incomplete" };
    expect(() => validateRuntimeBuildingRegistry(registry)).toThrow(/registry is blocked: Decode is incomplete/);
  });

  it("rejects unresolved semantic data even when the top-level gate claims ready", () => {
    const registry = runtimeReadyFixture();
    registry.buildings.rows[0].internalName = {
      state: "unresolved",
      confidence: "unknown",
      value: null,
      evidence: [],
      requiredEvidence: "Decode AdminBuildData name binding",
    };
    expect(() => validateRuntimeBuildingRegistry(registry)).toThrow(/unresolved data at buildings\.rows\[0\]\.internalName/);
  });

  it("rejects modified registry bytes", async () => {
    const payload = new TextEncoder().encode("verified");
    await expect(verifyRegistryBytes(payload, 8, "1c34f88707b55e6104c4eb20e71ffa3d33e414b71ef689a15fad0640d0ac58cb")).resolves.toBeUndefined();
    await expect(verifyRegistryBytes(new TextEncoder().encode("modified"), 8, "1c34f88707b55e6104c4eb20e71ffa3d33e414b71ef689a15fad0640d0ac58cb")).rejects.toThrow(/checksum mismatch/);
  });

  it("loads only the pinned registry path and verified payload", async () => {
    const payload = new TextEncoder().encode(JSON.stringify(runtimeReadyFixture()));
    const sha256 = await digest(payload);
    const fetchFn = vi.fn(async (path: string) => {
      if (path.endsWith("building-registry-manifest.json")) {
        return new Response(JSON.stringify({
          schemaVersion: 1,
          contractType: "building-registry-bootstrap",
          registryId: "evil-hunter-1.411.buildings-v1",
          registryPath: "/content/releases/evil-hunter-1.411/building-registry.json",
          registryBytes: payload.byteLength,
          registrySha256: sha256,
        }));
      }
      return new Response(payload);
    }) as unknown as typeof fetch;

    await expect(loadVerifiedBuildingRegistry(fetchFn)).resolves.toMatchObject({ runtimeState: "runtime-ready" });
    expect(fetchFn).toHaveBeenNthCalledWith(2, "/content/releases/evil-hunter-1.411/building-registry.json", {
      cache: "no-cache",
      credentials: "same-origin",
    });
  });

  it("loads a blocked registry only through the read-only evidence boundary", async () => {
    const registry = runtimeReadyFixture();
    registry.runtimeState = "blocked";
    registry.releaseGate = { runnable: false, blockingPaths: ["buildings.rows[0].capabilityIds.binding"], reason: "Controller dispatch unresolved" };
    const payload = new TextEncoder().encode(JSON.stringify(registry));
    const sha256 = await digest(payload);
    const fetchFn = vi.fn(async (path: string) => path.endsWith("building-registry-manifest.json")
      ? new Response(JSON.stringify({
        schemaVersion: 1,
        contractType: "building-registry-bootstrap",
        registryId: "evil-hunter-1.411.buildings-v1",
        registryPath: "/content/releases/evil-hunter-1.411/building-registry.json",
        registryBytes: payload.byteLength,
        registrySha256: sha256,
      }))
      : new Response(payload)) as unknown as typeof fetch;

    await expect(loadVerifiedBuildingEvidenceRegistry(fetchFn)).resolves.toMatchObject({ runtimeState: "blocked" });
    await expect(loadVerifiedBuildingRegistry(fetchFn)).rejects.toThrow(/registry is blocked/);
  });
});

function runtimeReadyFixture(): any {
  const evidence = [{ sourceId: SOURCE_ID, locator: "fixture", method: "serialized-row" }];
  const binding = () => ({ state: "resolved", confidence: "confirmed", evidence, requiredEvidence: null });
  const field = (value: unknown) => ({ ...binding(), value });
  const collection = (rows: unknown[] = []) => ({ binding: binding(), rows });
  return {
    schemaVersion: 1,
    contractType: "building-registry",
    registryId: "evil-hunter-1.411.buildings-v1",
    legacy: { game: "Evil Hunter Tycoon", version: "1.411", package: "com.superplanet.evilhunter" },
    runtimeState: "runtime-ready",
    evidencePolicy: {
      semanticFields: "evidence-required-per-field",
      unresolvedValues: "fail-closed-null-or-empty",
      visualBinding: "separate-from-gameplay-semantics",
    },
    evidenceSources: [{ id: SOURCE_ID, path: "reverse-engineering/evidence/source.json", bytes: 1, sha256: "0".repeat(64) }],
    catalogs: { items: collection(), products: collection(), capabilities: collection() },
    buildings: collection([{
      key: "decoded-building",
      buildId: field("build:decoded"),
      internalName: field("DecodedBuilding"),
      displayName: field({ en: "Decoded Building" }),
      category: field("decoded"),
      buildRows: collection(),
      levels: collection(),
      tradeRules: collection(),
      productIds: collection(),
      capabilityIds: collection(),
      visualBinding: {
        binding: binding(),
        spriteAssetId: field("sprite:decoded"),
        controllerClass: field("DecodedController"),
        popupClass: field("DecodedPop"),
        townPosition: field({ x: 1, y: 2 }),
        sorting: field({ layer: 0 }),
        collider: field({ width: 1, height: 1 }),
      },
    }]),
    releaseGate: { runnable: true, blockingPaths: [], reason: "All required evidence is resolved." },
  };
}

async function digest(payload: Uint8Array): Promise<string> {
  const buffer = new ArrayBuffer(payload.byteLength);
  new Uint8Array(buffer).set(payload);
  const hash = await crypto.subtle.digest("SHA-256", buffer);
  return [...new Uint8Array(hash)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}
