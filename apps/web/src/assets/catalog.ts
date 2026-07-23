import type { ContentAsset, ContentRelease, ContentUnit } from "../types";

interface RootManifest {
  manifestPath: string;
  manifestSha256: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseUnit(value: unknown): ContentUnit | null {
  if (!isRecord(value) || typeof value.id !== "string" || typeof value.kind !== "string" || !Array.isArray(value.assetIds)) return null;
  const assetIds = value.assetIds.filter((id): id is string => typeof id === "string");
  return {
    id: value.id,
    kind: value.kind,
    ...(typeof value.status === "string" ? { status: value.status } : {}),
    assetIds,
  };
}

function parseAsset(value: unknown): ContentAsset | null {
  if (!isRecord(value) || typeof value.id !== "string" || typeof value.unitId !== "string" || typeof value.publicPath !== "string") return null;
  return { id: value.id, unitId: value.unitId, publicPath: value.publicPath };
}

export async function loadContentRelease(): Promise<ContentRelease> {
  const rootResponse = await fetch("/content/manifest.json");
  if (!rootResponse.ok) throw new Error(`Content root manifest returned ${rootResponse.status}`);
  const root = (await rootResponse.json()) as Partial<RootManifest>;
  if (typeof root.manifestPath !== "string" || !root.manifestPath.startsWith("/content/") || typeof root.manifestSha256 !== "string") {
    throw new Error("Content root manifest is invalid");
  }

  const response = await fetch(root.manifestPath);
  if (!response.ok) throw new Error(`Content release manifest returned ${response.status}`);
  const manifestText = await response.text();
  if (!await matchesSha256(manifestText, root.manifestSha256)) throw new Error("Content release manifest checksum mismatch");
  const raw: unknown = JSON.parse(manifestText);
  if (!isRecord(raw) || typeof raw.releaseId !== "string" || !Array.isArray(raw.contentUnits) || !Array.isArray(raw.assets)) {
    throw new Error("Content release manifest is invalid");
  }

  const allUnits = raw.contentUnits.map(parseUnit).filter((unit): unit is ContentUnit => unit !== null);
  const approvedUnits = allUnits.filter((unit) => unit.status !== "unbound-candidate");
  const units = new Map(approvedUnits.map((unit) => [unit.id, unit]));
  const assets = raw.assets
    .map(parseAsset)
    .filter((asset): asset is ContentAsset => asset !== null && units.has(asset.unitId));
  return { releaseId: raw.releaseId, units, assets: new Map(assets.map((asset) => [asset.id, asset])) };
}

export async function matchesSha256(content: string, expectedHex: string): Promise<boolean> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(content));
  const actual = [...new Uint8Array(digest)].map((value) => value.toString(16).padStart(2, "0")).join("");
  return actual === expectedHex.toLowerCase();
}

export function requireContentAsset(release: ContentRelease, id: string): ContentAsset {
  const asset = release.assets.get(id);
  if (!asset) throw new Error(`Approved content asset is missing: ${id}`);
  return asset;
}
