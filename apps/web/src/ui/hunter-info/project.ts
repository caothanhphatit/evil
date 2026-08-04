import type { HunterView } from "../hunter-roster";
import type { HunterInfoEquipmentSlot, HunterInfoGrowthNode, HunterInfoMaterial, HunterInfoSkill, HunterInfoView, HunterInfoWeapon } from "./model";

type UnknownRecord = Record<string, unknown>;

export function projectHunterInfo(rawHunter: unknown, hunter: HunterView): HunterInfoView {
  const root = record(rawHunter);
  const info = record(root.hunter_info);
  const status = record(info.status);
  const progression = record(info.progression);
  const riding = record(info.riding_pet);
  // Characteristic is not the same domain as Job Trait; only the explicit field may prefix the name.
  const characteristic = text(info.characteristic_name);
  return {
    hunter,
    title: [characteristic, hunter.name].filter(Boolean).join(" ") || hunter.name,
    locked: boolean(info.locked),
    reincarnation: pair(info.reincarnation, "current", "maximum"),
    experience: pair(info.experience ?? progression.experience, "current", "maximum")
      ?? (hunter.level !== null ? pair({ current: root.xp, maximum: root.xp_to_next_level }, "current", "maximum") : null),
    dps: scaled(status.dps_milli, 1_000) ?? number(status.dps),
    criticalChance: scaled(status.critical_rate_bps, 100) ?? number(status.critical_chance),
    attackSpeed: scaled(status.attack_speed_milli, 1_000) ?? number(status.attack_speed),
    evasion: scaled(status.evasion_rate_bps, 100) ?? number(status.evasion),
    awakening: pair(status.awakening, "current", "maximum"),
    equipment: projectEquipmentSlots(info.equipment_slots),
    skills: projectSkills(info),
    growth: projectGrowth(info.growth),
    riding: projectRiding(riding, Object.prototype.hasOwnProperty.call(info, "riding_pet")),
    materials: projectMaterials(info.materials, Object.prototype.hasOwnProperty.call(info, "materials")),
    weapons: projectWeapons(info.weapons),
  };
}

const EQUIPMENT_SLOT_ORDER = ["gloves", "helmet", "necklace", "boots", "ring", "weapon", "armor", "belt"] as const;

function projectEquipmentSlots(value: unknown): HunterInfoEquipmentSlot[] {
  const projected = rows(value).map(projectEquipment).filter((entry): entry is HunterInfoEquipmentSlot => entry !== null);
  const byId = new Map(projected.map((entry) => [entry.id, entry]));
  return EQUIPMENT_SLOT_ORDER.map((id) => byId.get(id) ?? {
    id, catalogKind: null, catalogIndex: null, name: null, icon: null,
    placeholderIcon: null, presentationGender: null, requiredClassId: null,
    locked: null, evidenceState: null,
  });
}

function projectSkills(info: UnknownRecord): HunterInfoSkill[] | null {
  if (!Array.isArray(info.skills)) return null;
  return info.skills.map((value): HunterInfoSkill | null => {
    const row = record(value);
    const id = text(row.skill_id ?? row.id);
    const name = text(row.display_name ?? row.name);
    if (!id || !name) return null;
    return {
      id,
      name,
      icon: asset(row.icon_path ?? row.icon),
      level: number(row.level),
      description: text(row.description),
      group: text(row.group ?? row.tier),
      unlocked: boolean(row.unlocked),
      unlockRequirement: text(row.unlock_requirement),
      ready: boolean(row.ready),
      cooldownRemainingMs: number(row.cooldown_remaining_ms),
    };
  }).filter((skill): skill is HunterInfoSkill => skill !== null);
}

function projectGrowth(value: unknown): HunterInfoView["growth"] {
  const row = record(value);
  const secretPoints = number(row.secret_points);
  if (secretPoints === null) return null;
  const nodes = rows(row.nodes).map((entry, index): HunterInfoGrowthNode | null => {
    const node = record(entry);
    const id = text(node.node_id ?? node.id);
    const points = number(node.points);
    const maxPoints = number(node.max_points);
    if (!id || points === null || maxPoints === null) return null;
    return { id, icon: asset(node.icon_path ?? node.icon), points, maxPoints, order: number(node.order) ?? index };
  }).filter((entry): entry is HunterInfoGrowthNode => entry !== null).sort((a, b) => a.order - b.order);
  return { secretPoints, nodes };
}

function projectRiding(row: UnknownRecord, present: boolean): HunterInfoView["riding"] {
  if (!present) return null;
  const mounted = boolean(row.mounted);
  if (mounted === false) return { mounted: false, canMoveToRanch: boolean(row.can_move_to_ranch) === true };
  if (mounted === true) return { mounted: true, name: text(row.display_name ?? row.name), icon: asset(row.icon_path ?? row.icon) };
  return null;
}

function projectMaterials(value: unknown, present: boolean): HunterInfoMaterial[] | null {
  if (!present || !Array.isArray(value)) return null;
  return rows(value).map((entry, index): HunterInfoMaterial | null => {
    const row = record(entry);
    const id = text(row.material_id ?? row.id);
    const icon = asset(row.icon_path ?? row.icon);
    const quantity = number(row.quantity);
    if (!id || !icon || quantity === null) return null;
    return { id, icon, quantity, name: text(row.display_name ?? row.name), order: number(row.order) ?? index };
  }).filter((entry): entry is HunterInfoMaterial => entry !== null).sort((a, b) => a.order - b.order);
}

function projectWeapons(value: unknown): HunterInfoWeapon[] {
  return rows(value).map((entry): HunterInfoWeapon | null => {
    const row = record(entry);
    const instanceId = text(row.gear_instance_id);
    const productId = text(row.product_id);
    const weaponId = text(row.weapon_id);
    const nameEn = text(row.display_name_en);
    const nameVi = text(row.display_name_vi);
    const icon = asset(row.icon_path);
    const quality = number(row.quality);
    const attackDamage = number(row.attack_damage);
    const attackDamageMin = number(row.attack_damage_min);
    const attackDamageMax = number(row.attack_damage_max);
    const enhancementLevel = number(row.enhancement_level);
    const compatible = boolean(row.compatible);
    const equipped = boolean(row.equipped);
    const ruleset = text(row.ruleset);
    if (!instanceId || !productId || !weaponId || !nameEn || !nameVi || !icon || quality === null
      || attackDamage === null || attackDamageMin === null || attackDamageMax === null
      || enhancementLevel === null || compatible === null || equipped === null || !ruleset) return null;
    return { instanceId, productId, weaponId, nameEn, nameVi, icon, quality, attackDamage, attackDamageMin, attackDamageMax, enhancementLevel, compatible, equipped, ruleset };
  }).filter((entry): entry is HunterInfoWeapon => entry !== null);
}

function projectEquipment(value: unknown): HunterInfoEquipmentSlot | null {
  const row = record(value);
  const id = text(row.slot_id ?? row.id);
  if (!id) return null;
  return {
    id,
    catalogKind: text(row.catalog_kind),
    catalogIndex: number(row.catalog_index),
    name: text(row.display_name ?? row.name),
    icon: asset(row.icon_path ?? row.icon),
    placeholderIcon: asset(row.placeholder_icon_path),
    presentationGender: text(row.presentation_gender),
    requiredClassId: text(row.required_class_id),
    locked: boolean(row.locked),
    evidenceState: text(row.evidence_state),
  };
}

function pair(value: unknown, currentKey: string, maximumKey: string): { current: number; maximum: number } | null {
  const row = record(value);
  const current = number(row[currentKey]);
  const maximum = number(row[maximumKey]);
  return current === null || maximum === null ? null : { current, maximum };
}

function record(value: unknown): UnknownRecord { return typeof value === "object" && value !== null && !Array.isArray(value) ? value as UnknownRecord : {}; }
function rows(value: unknown): unknown[] { return Array.isArray(value) ? value : []; }
function text(value: unknown): string | null { return typeof value === "string" && value.trim() ? value.trim() : null; }
function number(value: unknown): number | null { const parsed = typeof value === "number" ? value : NaN; return Number.isFinite(parsed) ? parsed : null; }
function boolean(value: unknown): boolean | null { return typeof value === "boolean" ? value : null; }
function asset(value: unknown): string | null { const path = text(value); return path && (path.startsWith("/content/") || path.startsWith("/full-assets/")) ? path : null; }
function scaled(value: unknown, divisor: number): number | null { const amount = number(value); return amount === null ? null : amount / divisor; }
