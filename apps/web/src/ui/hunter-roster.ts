import type { OriginalFlowSnapshot } from "../generated/protocol";
import { t } from "../i18n";

export { hunterActorVisual } from "../game/hunter-actor-presentation";

export type HunterRosterState = "active" | "waiting";

export interface HunterSkillView {
  id: string;
  name: string;
  level: number | null;
  icon: string | null;
  ready: boolean | null;
}

export interface HunterTraitView {
  id: string;
  name: string;
  icon: string | null;
  rank: number | null;
  equipped: boolean | null;
}

export interface HunterView {
  id: string;
  numericId: number | null;
  name: string;
  rosterState: HunterRosterState;
  queuePosition: number | null;
  level: number | null;
  xp: number | null;
  classId: string | null;
  className: string | null;
  classFamily: string | null;
  rarityId: string | null;
  rarityName: string | null;
  traitName: string | null;
  traits: HunterTraitView[];
  action: string | null;
  animation: string | null;
  hp: number | null;
  maxHp: number | null;
  stamina: number | null;
  maxStamina: number | null;
  satiety: number | null;
  maxSatiety: number | null;
  mood: number | null;
  maxMood: number | null;
  attack: number | null;
  defense: number | null;
  gold: number | null;
  portrait: string | null;
  skills: HunterSkillView[];
  hunt: {
    status: "idle" | "hunting" | "returning" | "dead";
    zoneId: string | null;
    progressTicks: number;
    requiredTicks: number;
    loot: Array<{ itemId: string; quantity: number }>;
  } | null;
}

export interface HunterRosterView {
  capacity: number;
  active: HunterView[];
  waiting: HunterView[];
  selectedId: string | null;
  resolved: boolean;
  constraintViolation: string | null;
}

export type HunterRarityKey = "normal" | "rare" | "superior" | "heroic" | "legendary";

export interface HunterRarityPresentation {
  key: HunterRarityKey;
  letter: "N" | "R" | "S" | "H" | "L";
}

type UnknownRecord = Record<string, unknown>;

const DEFAULT_TOWN_CAPACITY = 8;

export function projectHunterRoster(snapshot: OriginalFlowSnapshot | UnknownRecord, selectedId: string | null = null): HunterRosterView {
  const root = record(snapshot);
  const roster = record(root.hunter_roster);
  const capacity = positiveInteger(roster.active_capacity) ?? DEFAULT_TOWN_CAPACITY;
  const activeRows = array(roster.active_hunters);
  const waitingRows = array(roster.waiting_hunters);
  const active = activeRows.flatMap((row, index) => {
    const hunter = parseHunter(row, "active", index);
    return hunter ? [hunter] : [];
  });
  const waiting = waitingRows.flatMap((row, index) => {
    const hunter = parseHunter(row, "waiting", index);
    return hunter ? [hunter] : [];
  });
  const activeIds = new Set(active.map((hunter) => hunter.id));
  const deDuplicatedWaiting = waiting.filter((hunter) => !activeIds.has(hunter.id));
  const resolved = activeRows.length > 0 || waitingRows.length > 0;
  return {
    capacity,
    active,
    waiting: deDuplicatedWaiting,
    selectedId: selectAvailableHunter(selectedId, active, deDuplicatedWaiting),
    resolved,
    constraintViolation: active.length > capacity ? t("roster.capacity_exceeded", { active: active.length, capacity }) : null,
  };
}

export function hunterPercent(current: number | null, maximum: number | null): number | null {
  if (current === null || maximum === null || maximum <= 0) return null;
  return Math.max(0, Math.min(100, Math.round((current / maximum) * 100)));
}

export function hunterRarityPresentation(rarityId: string | null, rarityName: string | null): HunterRarityPresentation | null {
  const rarity = `${rarityId ?? ""} ${rarityName ?? ""}`.trim().toLowerCase();
  if (!rarity) return null;
  if (/\blegendary\b|^l$/.test(rarity)) return { key: "legendary", letter: "L" };
  if (/\bheroic\b|^h$/.test(rarity)) return { key: "heroic", letter: "H" };
  if (/\bsuperior\b|^s$/.test(rarity)) return { key: "superior", letter: "S" };
  if (/\brare\b|^r$/.test(rarity)) return { key: "rare", letter: "R" };
  if (/\bnormal\b|^n$/.test(rarity)) return { key: "normal", letter: "N" };
  return null;
}

export function hunterClassTone(classFamily: string | null): string {
  return classFamily && /^H[1-5]$/.test(classFamily) ? classFamily.toLowerCase() : "unresolved";
}

export function hunterWorldEntityId(snapshot: OriginalFlowSnapshot | UnknownRecord, hunter: HunterView): string | null {
  const root = record(snapshot);
  const world = record(root.world);
  const entities = array(world.entities).map(record);
  const direct = entities.find((entity) => entity.descriptor && record(entity.descriptor).entity_id === hunter.id);
  if (direct && direct.selectable === true) return stringValue(record(direct.descriptor).entity_id);
  if (hunter.numericId === null) return null;
  const match = entities.find((entity) => {
    const descriptor = record(entity.descriptor);
    return entity.selectable === true
      && descriptor.source_skeleton_name === "hunter"
      && stringValue(descriptor.entity_id)?.match(/(\d+)$/)?.[1] === String(hunter.numericId);
  });
  return match ? stringValue(record(match.descriptor).entity_id) : null;
}

function parseHunter(value: unknown, rosterState: HunterRosterState, index: number): HunterView | null {
  const row = record(value);
  const hunt = record(row.hunt);
  const traits = array(row.traits).flatMap((trait, traitIndex) => {
    const parsed = parseTrait(trait, traitIndex);
    return parsed ? [parsed] : [];
  });
  const numericId = finiteNumber(row.hunter_id);
  const displayName = stringValue(row.display_name);
  if (numericId === null || displayName === null) return null;
  const id = `hunter-${numericId}`;
  return {
    id,
    numericId,
    name: displayName,
    rosterState,
    queuePosition: rosterState === "waiting" ? index + 1 : null,
    level: finiteNumber(row.level),
    xp: finiteNumber(row.xp),
    classId: stringValue(row.class_id),
    className: stringValue(row.class_name),
    classFamily: normalizeClassFamily(row.class_family),
    rarityId: stringValue(row.rarity_id),
    rarityName: stringValue(row.rarity_name),
    traitName: stringValue(row.trait_name)
      ?? (traits.filter((candidate) => candidate.equipped !== false).map((candidate) => candidate.name).join(", ") || null),
    traits,
    action: stringValue(row.action_state),
    animation: stringValue(row.animation),
    hp: finiteNumber(row.current_hp),
    maxHp: finiteNumber(row.max_hp),
    stamina: finiteNumber(row.stamina),
    maxStamina: finiteNumber(row.max_stamina),
    satiety: finiteNumber(row.satiety),
    maxSatiety: finiteNumber(row.max_satiety),
    mood: finiteNumber(row.mood),
    maxMood: finiteNumber(row.max_mood),
    attack: finiteNumber(row.attack),
    defense: finiteNumber(row.defense),
    gold: finiteNumber(row.gold),
    portrait: safeAssetPath(row.portrait_asset_id),
    skills: array(row.skills).flatMap((skill, skillIndex) => {
      const parsed = parseSkill(skill, skillIndex);
      return parsed ? [parsed] : [];
    }),
    hunt: projectHunt(hunt),
  };
}

function projectHunt(row: UnknownRecord): HunterView["hunt"] {
  const status = stringValue(row.status);
  const progressTicks = finiteNumber(row.progress_ticks);
  const requiredTicks = finiteNumber(row.required_ticks);
  if (!status || !["idle", "hunting", "returning", "dead"].includes(status) || progressTicks === null || requiredTicks === null) return null;
  return {
    status: status as NonNullable<HunterView["hunt"]>["status"],
    zoneId: stringValue(row.zone_id),
    progressTicks,
    requiredTicks,
    loot: array(row.loot).map(record).flatMap((loot) => {
      const itemId = stringValue(loot.item_id);
      const quantity = finiteNumber(loot.quantity);
      return itemId && quantity !== null ? [{ itemId, quantity }] : [];
    }),
  };
}

function parseSkill(value: unknown, _index: number): HunterSkillView | null {
  const row = record(value);
  const id = stringValue(row.skill_id);
  const name = stringValue(row.display_name);
  if (!id || !name) return null;
  return {
    id,
    name,
    level: finiteNumber(row.level),
    icon: safeAssetPath(row.icon_path),
    ready: typeof row.ready === "boolean" ? row.ready : null,
  };
}

function parseTrait(value: unknown, _index: number): HunterTraitView | null {
  const row = record(value);
  const id = stringValue(row.trait_id);
  const name = stringValue(row.display_name);
  if (!id || !name) return null;
  return {
    id,
    name,
    icon: safeAssetPath(row.icon_path),
    rank: finiteNumber(row.unlocked_rank),
    equipped: typeof row.equipped === "boolean" ? row.equipped : null,
  };
}

function selectAvailableHunter(selectedId: string | null, active: HunterView[], waiting: HunterView[]): string | null {
  if (selectedId && [...active, ...waiting].some((hunter) => hunter.id === selectedId)) return selectedId;
  return active[0]?.id ?? waiting[0]?.id ?? null;
}

function normalizeClassFamily(value: unknown): string | null {
  const text = stringValue(value)?.toUpperCase();
  return text && /^H[1-5]$/.test(text) ? text : null;
}

function safeAssetPath(value: unknown): string | null {
  const path = stringValue(value);
  if (!path || (!path.startsWith("/content/") && !path.startsWith("/full-assets/"))) return null;
  return path;
}

function record(value: unknown): UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? value as UnknownRecord : {};
}

function array(value: unknown): unknown[] { return Array.isArray(value) ? value : []; }
function stringValue(value: unknown): string | null { return typeof value === "string" && value.trim() ? value.trim() : null; }
function finiteNumber(value: unknown): number | null {
  const number = typeof value === "number" ? value : typeof value === "string" && value.trim() ? Number(value) : NaN;
  return Number.isFinite(number) ? number : null;
}
function positiveInteger(value: unknown): number | null {
  const number = finiteNumber(value);
  return number !== null && number > 0 ? Math.floor(number) : null;
}
