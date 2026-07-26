import type { EvidenceBuildingRegistry, EvidenceField } from "./building-registry";

export type BuildingPopupRoute = "building" | "request" | "production" | "service";

export interface BuildingEvidenceView {
  id: string;
  name: string;
  description: string;
  maxLevel: number | null;
  levels: Array<{ level: number; costs: string[]; requiredTownHallLevel: number | null }>;
  maxBuild: number | null;
  gridSize: [number, number] | null;
  popupRoute: BuildingPopupRoute | null;
  capabilityKinds: string[];
  actionBlockedReason: string | null;
  constructionBlockedReason: string | null;
  spriteAssetId: string | null;
}

export function projectBuildingEvidence(
  registry: EvidenceBuildingRegistry | null,
  buildingId: string,
  locale = navigatorLanguage(),
): BuildingEvidenceView | null {
  if (!registry) return null;
  const building = registry.buildings.rows.find((row) => resolvedString(row.buildId) === buildingId);
  if (!building) return null;

  const levels = collectionRows(building.levels).flatMap((row) => {
    const level = resolvedNumber(row.level);
    if (level === null) return [];
    const condition = collectionRows(row.conditions).find((entry) => resolvedString(entry.subjectId) === "build_1.level"
      && resolvedString(entry.operator) === "greater-than-or-equal");
    return [{
      level,
      costs: collectionRows(row.upgradeCosts).flatMap((amount) => formatAmount(amount, registry, locale)),
      requiredTownHallLevel: condition ? resolvedNumber(condition.operand) : null,
    }];
  });
  const capabilities = capabilitiesFor(registry, building);
  const capabilityKinds = capabilities.flatMap((capability) => {
    const kind = resolvedString(capability.kind);
    return kind ? [kind] : [];
  });
  const popupRoute = resolvePopupRoute(buildingId, building, capabilityKinds, collectionRows(building.productIds).length > 0);
  const capabilityDescription = capabilities.flatMap((capability) => {
    const parameters = record(resolvedValue(capability.parameters));
    const description = localizedObjectValue(parameters?.description, locale);
    return description ? [description] : [];
  })[0] ?? null;
  const capabilityBinding = record(building.capabilityIds)?.binding;
  const popupField = record(building.visualBinding)?.popupClass;
  const placementField = record(building.visualBinding)?.townPosition;
  const targetTimer = collectionRows(building.buildRows)[0]?.durationMs;
  const unresolvedReason = requiredEvidence(popupField)
    ?? requiredEvidence(capabilityBinding)
    ?? "No decoded popup or capability is bound to this building.";

  return {
    id: buildingId,
    name: localizedValue(building.displayName, locale) ?? buildingId,
    description: capabilityDescription ?? localizedValue(building.description, locale) ?? "Original description is not yet present in the evidence contract.",
    maxLevel: levels.length ? Math.max(...levels.map((entry) => entry.level)) : null,
    levels,
    maxBuild: resolvedNumber(record(building.sourceData)?.maxBuild),
    gridSize: resolvedNumberPair(record(building.sourceData)?.gridSize),
    popupRoute,
    capabilityKinds,
    actionBlockedReason: popupRoute ? null : unresolvedReason,
    constructionBlockedReason: requiredEvidence(placementField) ?? requiredEvidence(targetTimer),
    spriteAssetId: resolvedString(record(building.visualBinding)?.spriteAssetId),
  };
}

export function listBuildingEvidence(registry: EvidenceBuildingRegistry | null, locale = navigatorLanguage()): BuildingEvidenceView[] {
  if (!registry) return [];
  return registry.buildings.rows.flatMap((row) => {
    const id = resolvedString(row.buildId);
    return id ? [projectBuildingEvidence(registry, id, locale)!] : [];
  });
}

export function formatLevelCosts(view: BuildingEvidenceView, level: number): string {
  const costs = view.levels.find((entry) => entry.level === level)?.costs ?? [];
  return costs.length ? costs.join(" · ") : "Cost unresolved";
}

function resolvePopupRoute(buildingId: string, building: Record<string, unknown>, capabilityKinds: string[], hasProducts: boolean): BuildingPopupRoute | null {
  const popupClass = resolvedString(record(building.visualBinding)?.popupClass);
  return popupClassRoute(buildingId, popupClass, capabilityKinds, hasProducts);
}

function capabilitiesFor(registry: EvidenceBuildingRegistry, building: Record<string, unknown>): Array<Record<string, unknown>> {
  const capabilityIds = new Set(collectionRows(building.capabilityIds).flatMap((row) => {
    const id = resolvedString(row.id);
    return id ? [id] : [];
  }));
  return registry.catalogs.capabilities.rows.filter((capability) => {
    const capabilityId = resolvedString(capability.capabilityId);
    return capabilityId !== null && capabilityIds.has(capabilityId);
  });
}

function popupClassRoute(buildingId: string, popupClass: string | null, capabilityKinds: string[], hasProducts: boolean): BuildingPopupRoute | null {
  if (new Set(["build_2", "build_9", "build_12", "build_13", "build_19", "build_24", "build_25", "build_26", "build_27", "build_28"]).has(buildingId)) return "service";
  // RequestPop is the bounty dialog. Trading Post requests are a BuildingPop mode.
  if (popupClass === "BuildingPop" && capabilityKinds.includes("loot-purchase-reservations")) return "request";
  // The decoded weapon/armor capability includes the complete recipe contract,
  // so its GearCreate flow is safe to expose even when the prefab popup field is absent.
  if (hasProducts || capabilityKinds.some((kind) => kind.includes("crafting") || kind.includes("display-and-sale"))) return "production";
  if (popupClass === "GearCreatePop") return "production";
  if (popupClass === "BuildingPop") return "building";
  return null;
}

function formatAmount(row: Record<string, unknown>, registry: EvidenceBuildingRegistry, locale: string): string[] {
  const itemId = resolvedString(row.itemId);
  const quantity = resolvedNumber(row.quantity);
  if (!itemId || quantity === null) return [];
  return [`${itemDisplayName(registry, itemId, locale)} ${quantity.toLocaleString(locale)}`];
}

function itemDisplayName(registry: EvidenceBuildingRegistry, itemId: string, locale: string): string {
  if (itemId === "currency:gold") return "Gold";
  const item = registry.catalogs.items.rows.find((row) => resolvedString(row.itemId) === itemId);
  const displayName = localizedValue(item?.displayName, locale);
  if (displayName) return displayName;
  return itemId.replace(":", " ");
}

function resolvedNumberPair(value: unknown): [number, number] | null {
  const resolved = resolvedValue(value);
  return Array.isArray(resolved) && resolved.length === 2 && resolved.every((entry) => typeof entry === "number" && Number.isFinite(entry))
    ? [resolved[0] as number, resolved[1] as number]
    : null;
}

function localizedValue(value: unknown, locale: string): string | null {
  const fieldValue = resolvedValue(value);
  if (typeof fieldValue === "string") return fieldValue;
  return localizedObjectValue(fieldValue, locale);
}

function localizedObjectValue(value: unknown, locale: string): string | null {
  const translations = record(value);
  if (!translations) return null;
  const normalized = locale.replace("_", "-");
  const language = normalized.split("-")[0];
  for (const key of [normalized, language, "en"]) {
    if (typeof translations[key] === "string" && translations[key].length > 0) return translations[key];
  }
  return null;
}

function resolvedString(value: unknown): string | null {
  const resolved = resolvedValue(value);
  return typeof resolved === "string" && resolved.length > 0 ? resolved : null;
}

function resolvedNumber(value: unknown): number | null {
  const resolved = resolvedValue(value);
  return typeof resolved === "number" && Number.isFinite(resolved) ? resolved : null;
}

function resolvedValue(value: unknown): unknown {
  const field = record(value) as EvidenceField | null;
  return field?.state === "resolved" ? field.value : null;
}

function requiredEvidence(value: unknown): string | null {
  const binding = record(value);
  return binding?.state === "unresolved" && typeof binding.requiredEvidence === "string" ? binding.requiredEvidence : null;
}

function collectionRows(value: unknown): Array<Record<string, unknown>> {
  const rows = record(value)?.rows;
  return Array.isArray(rows) ? rows.filter((row): row is Record<string, unknown> => record(row) !== null) : [];
}

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? value as Record<string, unknown> : null;
}

function navigatorLanguage(): string {
  return typeof navigator === "undefined" ? "en" : navigator.language;
}
