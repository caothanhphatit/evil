import type { OriginalFlowSnapshot, WorldEntityProjection } from "../generated/protocol";

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
}

export interface HunterRosterView {
  capacity: number;
  active: HunterView[];
  waiting: HunterView[];
  selectedId: string | null;
  resolved: boolean;
  constraintViolation: string | null;
}

type UnknownRecord = Record<string, unknown>;

const DEFAULT_TOWN_CAPACITY = 8;

export function projectHunterRoster(snapshot: OriginalFlowSnapshot | UnknownRecord, selectedId: string | null = null): HunterRosterView {
  const root = record(snapshot);
  const roster = record(root.hunter_roster);
  const world = record(root.world);
  const capacity = positiveInteger(roster.active_capacity ?? roster.capacity ?? roster.max_active_hunters) ?? DEFAULT_TOWN_CAPACITY;
  const activeRows = array(roster.active_hunters ?? roster.hunters ?? roster.active ?? roster.roster);
  const waitingRows = array(roster.waiting_queue ?? roster.waiting_hunters ?? roster.waiting ?? roster.queue);
  const fallbackRows = activeRows.length === 0 ? fallbackHunterRows(roster, array(world.entities)) : [];
  const active = uniqueHunters([...activeRows, ...fallbackRows].map((row, index) => parseHunter(row, "active", index)));
  const waiting = uniqueHunters(waitingRows.map((row, index) => parseHunter(row, "waiting", index)));
  const activeIds = new Set(active.map((hunter) => hunter.id));
  const deDuplicatedWaiting = waiting.filter((hunter) => !activeIds.has(hunter.id));
  const resolved = activeRows.length > 0 || waitingRows.length > 0 || roster.roster_resolved === true;
  return {
    capacity,
    active,
    waiting: deDuplicatedWaiting,
    selectedId: selectAvailableHunter(selectedId, active, deDuplicatedWaiting),
    resolved,
    constraintViolation: active.length > capacity ? `Town capacity exceeded: ${active.length}/${capacity}` : null,
  };
}

export function hunterPercent(current: number | null, maximum: number | null): number | null {
  if (current === null || maximum === null || maximum <= 0) return null;
  return Math.max(0, Math.min(100, Math.round((current / maximum) * 100)));
}

function parseHunter(value: unknown, rosterState: HunterRosterState, index: number): HunterView {
  const row = record(value);
  const profile = record(row.profile ?? row.hunter_profile);
  const stats = record(row.stats ?? row.attributes ?? row.vitals ?? profile.stats);
  const action = record(row.action ?? row.action_state ?? profile.action);
  const identity = record(row.identity);
  const job = record(row.class ?? row.job ?? profile.class);
  const trait = record(row.trait);
  const traits = array(row.traits ?? profile.traits).map(parseTrait);
  const idValue = row.hunter_id ?? row.id ?? identity.id;
  const numericId = finiteNumber(idValue);
  const id = stringValue(row.entity_id) ?? (numericId === null ? `hunter-${rosterState}-${index + 1}` : `hunter-${numericId}`);
  return {
    id,
    numericId,
    name: stringValue(row.display_name ?? row.name ?? profile.display_name ?? identity.name) ?? `Hunter ${numericId ?? index + 1}`,
    rosterState,
    queuePosition: rosterState === "waiting" ? positiveInteger(row.queue_position) ?? index + 1 : null,
    level: finiteNumber(row.level ?? profile.level ?? stats.level),
    xp: finiteNumber(row.xp ?? profile.xp),
    classId: stringValue(row.class_id ?? row.job_id ?? profile.class_id ?? job.id),
    className: stringValue(row.class_name ?? row.job_name ?? profile.class_name ?? job.name),
    classFamily: normalizeClassFamily(row.class_family ?? row.job_family ?? profile.visual_family ?? profile.class_family ?? job.family),
    rarityId: stringValue(row.rarity_id ?? profile.rarity_id),
    rarityName: stringValue(row.rarity_name ?? profile.rarity_name),
    traitName: stringValue(row.trait_name ?? trait.name ?? (typeof row.trait === "string" ? row.trait : null))
      ?? (traits.filter((candidate) => candidate.equipped !== false).map((candidate) => candidate.name).join(", ") || null),
    traits,
    action: stringValue(action.kind ?? action.state ?? row.action_state ?? profile.action_state ?? row.state),
    animation: stringValue(action.animation ?? row.animation ?? profile.animation_name ?? profile.animation),
    hp: finiteNumber(stats.hp ?? stats.current_hp ?? row.hp ?? row.current_hp),
    maxHp: finiteNumber(stats.max_hp ?? row.max_hp),
    stamina: finiteNumber(stats.stamina ?? stats.current_stamina ?? row.stamina),
    maxStamina: finiteNumber(stats.max_stamina ?? row.max_stamina),
    satiety: finiteNumber(stats.satiety ?? stats.current_satiety ?? row.satiety),
    maxSatiety: finiteNumber(stats.max_satiety ?? row.max_satiety),
    mood: finiteNumber(stats.mood ?? stats.current_mood ?? row.mood),
    maxMood: finiteNumber(stats.max_mood ?? row.max_mood),
    attack: finiteNumber(stats.attack ?? stats.attack_power ?? row.attack ?? profile.attack),
    defense: finiteNumber(stats.defense ?? row.defense ?? profile.defense),
    gold: finiteNumber(row.gold ?? stats.gold),
    portrait: safeAssetPath(row.portrait ?? row.portrait_path ?? row.portrait_asset ?? profile.portrait_asset_id),
    skills: array(row.skills ?? profile.skills).map(parseSkill),
  };
}

function parseSkill(value: unknown, index: number): HunterSkillView {
  const row = record(value);
  return {
    id: stringValue(row.id ?? row.skill_id) ?? `skill-${index + 1}`,
    name: stringValue(row.display_name ?? row.name) ?? `Skill ${index + 1}`,
    level: finiteNumber(row.level ?? row.skill_level),
    icon: safeAssetPath(row.icon ?? row.icon_path),
    ready: typeof row.ready === "boolean" ? row.ready : null,
  };
}

function parseTrait(value: unknown, index: number): HunterTraitView {
  const row = record(value);
  return {
    id: stringValue(row.id ?? row.trait_id) ?? `trait-${index + 1}`,
    name: stringValue(row.display_name ?? row.name) ?? `Trait ${index + 1}`,
    icon: safeAssetPath(row.icon ?? row.icon_path),
    rank: finiteNumber(row.rank ?? row.unlocked_rank),
    equipped: typeof row.equipped === "boolean" ? row.equipped : null,
  };
}

function fallbackHunterRows(roster: UnknownRecord, worldEntities: unknown[]): unknown[] {
  const candidates: unknown[] = [];
  const infirmary = record(roster.infirmary);
  candidates.push(...array(infirmary.hunters));
  for (const service of array(roster.product_services)) candidates.push(...array(record(service).hunters));
  for (const entity of worldEntities) {
    const row = record(entity);
    if (record(row.descriptor).kind === "hunter") candidates.push(entityRow(row));
  }
  return candidates;
}

function entityRow(row: UnknownRecord): UnknownRecord {
  const descriptor = record(row.descriptor);
  const entityId = stringValue(descriptor.entity_id);
  const numericId = entityId?.match(/(\d+)$/)?.[1];
  return { ...row, entity_id: entityId, hunter_id: numericId ? Number(numericId) : undefined };
}

function uniqueHunters(hunters: HunterView[]): HunterView[] {
  const result = new Map<string, HunterView>();
  for (const hunter of hunters) {
    const previous = result.get(hunter.id);
    result.set(hunter.id, previous ? mergeHunter(previous, hunter) : hunter);
  }
  return [...result.values()];
}

function mergeHunter(first: HunterView, second: HunterView): HunterView {
  const merged = { ...first };
  for (const key of Object.keys(second) as Array<keyof HunterView>) {
    const value = second[key];
    if (value !== null && value !== "" && (!(Array.isArray(value)) || value.length > 0)) (merged as UnknownRecord)[key] = value;
  }
  return merged;
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

const FAMILY_SKINS: Record<string, string[]> = {
  H1: ["All_h1", "All_h1_duallist"],
  H2: ["All_h2", "All_h2_executor", "All_h2_templer"],
  H3: ["All_h3", "All_h3_mistic"],
  H4: ["All_h4", "All_h4_darkload"],
  H5: ["All_h5", "All_h5_concentrate"],
};
const DEMO_SKINS = ["All_h1", "All_h1_duallist", "All_h2", "All_h2_executor", "All_h3", "All_h3_mistic", "All_h4", "All_h5"];
const HUNTER_TINTS = [0xffffff, 0xfff4dd, 0xe8f5ff, 0xf2e9ff, 0xe8ffe9, 0xffe9ec, 0xfff8cc, 0xe7ffff];

export function hunterActorVisual(entity: WorldEntityProjection | UnknownRecord): { skinNames: string[]; animation: string | null; tint: number; signature: string } {
  const row = record(entity);
  const descriptor = record(row.descriptor);
  const profile = record(row.profile ?? row.hunter_profile ?? descriptor.profile ?? descriptor.hunter_profile);
  const visual = record(row.hunter_visual ?? row.visual ?? profile.visual ?? descriptor.hunter_visual ?? descriptor.visual);
  const family = normalizeClassFamily(visual.class_family ?? visual.visual_family ?? row.class_family ?? profile.visual_family ?? profile.class_family ?? descriptor.class_family);
  const variant = stableHunterVariant(descriptor.entity_id ?? row.entity_id ?? row.hunter_id ?? row.id);
  const explicitSkins = array(visual.skin_names).filter((value): value is string => typeof value === "string" && value.length > 0);
  const weaponSkin = stringValue(visual.weapon_skin ?? profile.weapon_skin);
  const familySkins = family ? FAMILY_SKINS[family] : null;
  const skinNames = explicitSkins.length > 0 ? explicitSkins : [familySkins?.[variant % familySkins.length] ?? DEMO_SKINS[variant]];
  if (weaponSkin) skinNames.push(weaponSkin);
  const animation = stringValue(visual.animation ?? visual.animation_name ?? profile.animation_name ?? profile.animation ?? row.animation);
  const tint = HUNTER_TINTS[variant];
  return { skinNames, animation, tint, signature: `${skinNames.join("|")}:${tint.toString(16)}` };
}

function stableHunterVariant(value: unknown): number {
  const text = stringValue(value) ?? "hunter-1";
  const numericSuffix = text.match(/(\d+)$/)?.[1];
  if (numericSuffix) return Math.max(0, Number(numericSuffix) - 1) % DEMO_SKINS.length;
  let hash = 0;
  for (const character of text) hash = ((hash * 31) + character.charCodeAt(0)) >>> 0;
  return hash % DEMO_SKINS.length;
}
