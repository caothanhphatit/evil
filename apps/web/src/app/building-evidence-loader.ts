import type { OriginalFlowSnapshot } from "../generated/protocol";
import { recordClientEvent } from "../observability/client-telemetry";
import { decodeGearCatalog, loadGearCatalog } from "../content/blacksmith-route";
import { loadVerifiedBuildingEvidenceRegistry } from "../content/building-registry";
import type { BuildingRenderingContext } from "./building-renderer";
import type { VisibleEntityWorld } from "../game/visible-world";

export async function initializeBuildingEvidence(options: {
  context: BuildingRenderingContext;
  world: VisibleEntityWorld | null;
  snapshot: OriginalFlowSnapshot | null;
  debugUi: boolean;
  render: (snapshot: OriginalFlowSnapshot | null) => void;
  sync: (world: VisibleEntityWorld, snapshot: OriginalFlowSnapshot) => void;
  fallbackMessage: string;
}): Promise<void> {
  try {
    const [registry, decodedCatalog] = await Promise.all([loadVerifiedBuildingEvidenceRegistry(), loadGearCatalog().catch(() => null)]);
    options.context.buildingEvidenceRegistry = registry;
    options.context.gearCatalog = decodedCatalog ?? decodeGearCatalog(registry.catalogs.items.rows, registry.catalogs.products.rows);
    options.context.gearMaterialIcons.clear();
    for (const recipe of options.context.gearCatalog) {
      for (const cost of recipe.materialCosts) if (cost.iconPath) options.context.gearMaterialIcons.set(cost.materialId, cost.iconPath);
    }
    options.context.buildingEvidenceError = null;
    if (options.world && options.snapshot) options.sync(options.world, options.snapshot);
    options.render(options.snapshot);
  } catch (error) {
    recordClientEvent("error", "building_evidence_load_failed", { reason: error instanceof Error ? error.message : "unknown" });
    options.context.buildingEvidenceError = options.debugUi && error instanceof Error ? error.message : options.fallbackMessage;
    console.error("Failed to load verified building evidence.", error);
  }
}
