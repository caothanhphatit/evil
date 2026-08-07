import type { WorldEntityProjection } from "../generated/protocol";
import { hunterBaseWeaponSkin } from "./hunter-spine-presentation";

type UnknownRecord = Record<string, unknown>;

export interface HunterActorVisual {
  skinNames: string[];
  animation: string | null;
  tint: number;
  signature: string;
}

// `All_h*` are hero/demo compositions; ordinary bodies and outfits are separate skins.
const FAMILY_BODY_SKINS: Record<string, [string, string]> = {
  H1: ["hunter_m_01", "hunter_f_01"],
  H2: ["hunter_m_01", "hunter_f_01"],
  H3: ["hunter_m_01", "hunter_f_01"],
  H4: ["hunter_m_01", "hunter_f_01"],
  H5: ["hunter_m_01", "hunter_f_01"],
};
const FAMILY_COSTUME_SKINS: Record<string, [string, string]> = {
  H1: ["costum_h1_01", "costum_h1_02"],
  H2: ["costum_h2_01", "costum_h2_02"],
  H3: ["costum_h3_01", "costum_h3_02"],
  H4: ["costum_h4_01", "costum_h4_02"],
  H5: ["costum_h5_01", "costum_h5_02"],
};
const FAMILY_PAPER_DOLL_SKINS: Record<string, string> = {
  H1: "All_h1",
  H2: "All_h2",
  H3: "All_h3",
  H4: "All_h4",
  H5: "All_h5",
};
const HUNTER_TINTS = [0xffffff, 0xfff4dd, 0xe8f5ff, 0xf2e9ff, 0xe8ffe9, 0xffe9ec, 0xfff8cc, 0xe7ffff];

export function hunterActorVisual(entity: WorldEntityProjection | UnknownRecord): HunterActorVisual {
  const row = record(entity);
  const descriptor = record(row.descriptor);
  const profile = record(row.profile ?? row.hunter_profile ?? descriptor.profile ?? descriptor.hunter_profile);
  const visual = record(row.hunter_visual ?? row.visual ?? profile.visual ?? descriptor.hunter_visual ?? descriptor.visual);
  const family = normalizeClassFamily(visual.class_family ?? visual.visual_family ?? row.class_family ?? profile.visual_family ?? profile.class_family ?? descriptor.class_family);
  const variant = stableHunterVariant(descriptor.entity_id ?? row.entity_id ?? row.hunter_id ?? row.id);
  const rawAnimation = stringValue(visual.animation ?? visual.animation_name ?? profile.animation_name ?? profile.animation ?? row.animation);
  const animation = rawAnimation?.endsWith("_skill")
    ? `${family?.toLowerCase() ?? "hunter"}_hit`
    : rawAnimation;
  const tint = HUNTER_TINTS[variant];
  const explicitSkins = array(visual.skin_names).filter((value): value is string => typeof value === "string" && value.length > 0);
  const weaponSkin = stringValue(visual.weapon_skin ?? profile.weapon_skin) ?? hunterBaseWeaponSkin(family);
  const bodySkins = family ? FAMILY_BODY_SKINS[family] : null;
  const costumeSkins = family ? FAMILY_COSTUME_SKINS[family] : null;
  const genderIndex = variant % 2;
  const skinNames = explicitSkins.length > 0
    ? explicitSkins
    : bodySkins && costumeSkins
      ? [bodySkins[genderIndex], costumeSkins[genderIndex]]
      : [];
  if (skinNames.length === 0) return { skinNames: [], animation, tint, signature: `unresolved:${tint.toString(16)}` };
  if (weaponSkin) skinNames.push(weaponSkin);
  return { skinNames, animation, tint, signature: `${skinNames.join("|")}:${tint.toString(16)}` };
}

export function hunterPaperDollVisual(entity: WorldEntityProjection | UnknownRecord): HunterActorVisual {
  const visual = hunterActorVisual(entity);
  const row = record(entity);
  const profile = record(row.profile ?? row.hunter_profile);
  const family = normalizeClassFamily(row.class_family ?? profile.visual_family ?? profile.class_family);
  const aggregateSkin = family ? FAMILY_PAPER_DOLL_SKINS[family] : null;
  return aggregateSkin
    ? { ...visual, skinNames: [aggregateSkin], signature: `${aggregateSkin}:${visual.tint.toString(16)}` }
    : visual;
}

function normalizeClassFamily(value: unknown): string | null {
  const family = stringValue(value)?.toUpperCase() ?? null;
  return family && /^H[1-5]$/.test(family) ? family : null;
}

function stableHunterVariant(value: unknown): number {
  const text = stringValue(value) ?? "hunter-1";
  const numericSuffix = text.match(/(\d+)$/)?.[1];
  if (numericSuffix) return Math.max(0, Number(numericSuffix) - 1) % HUNTER_TINTS.length;
  let hash = 0;
  for (const character of text) hash = ((hash * 31) + character.charCodeAt(0)) >>> 0;
  return hash % HUNTER_TINTS.length;
}

function record(value: unknown): UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? value as UnknownRecord : {};
}

function array(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function stringValue(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}
