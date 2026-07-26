export interface HunterAssetEvidence {
  sourceId: string;
  locator: string;
  method: string;
  confidence: "confirmed";
  note: string;
}

export interface HunterFileAsset {
  id?: string;
  sourcePath: string;
  publicPath: string;
  bytes: number;
  sha256: string;
  evidence: HunterAssetEvidence;
}

export interface HunterAssetCatalog {
  schemaVersion: 1;
  catalogId: string;
  counts: Record<string, number>;
  spineBundle: HunterFileAsset[];
  portraits: Array<HunterFileAsset & { family: "female" | "male"; index: number; semanticBinding: "unresolved" }>;
  visualCatalog: {
    aggregateSkins: Array<{ name: string; visualFamily: string; compositionBinding: "unresolved"; evidence: HunterAssetEvidence }>;
    weaponSkins: Array<{ name: string; visualFamily: string; equipmentBinding: "unresolved"; evidence: HunterAssetEvidence }>;
    animations: Array<{ name: string; visualFamily: string; gameplaySemantics: "unresolved"; evidence: HunterAssetEvidence }>;
  };
  uiAssets: Record<string, Array<HunterFileAsset & { sourceName: string; semanticBinding: "unresolved" }>>;
}

export async function loadHunterAssetCatalog(fetchFn: typeof fetch = fetch): Promise<HunterAssetCatalog> {
  const response = await fetchFn("/content/releases/evil-hunter-1.411/hunter-assets/catalog.json", { cache: "no-cache", credentials: "same-origin" });
  if (!response.ok) throw new Error(`Hunter asset catalog returned ${response.status}`);
  const payload = await response.json() as unknown;
  if (!isRecord(payload) || payload.schemaVersion !== 1 || payload.catalogId !== "evil-hunter-1.411-hunter-assets-v1"
    || !Array.isArray(payload.spineBundle) || !Array.isArray(payload.portraits) || !isRecord(payload.visualCatalog) || !isRecord(payload.uiAssets)) {
    throw new Error("Hunter asset catalog structure is invalid");
  }
  return payload as unknown as HunterAssetCatalog;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
