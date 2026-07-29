import type { FixtureCombatWorldSnapshot, MonsterWorldSnapshot, WorldEntityProjection } from "../generated/protocol";

/**
 * Map and spawn rows are presentation fixtures until a server projection binds
 * the original map ids/rates. Keeping them explicit prevents the UI from
 * presenting guessed legacy balance as authoritative data.
 */
export const MONSTER_MAP_FIXTURES = [
  { id: "map_new01", label: "Monster Tier I", assetPath: "/content/releases/visible-world-v1/maps/map_new01.png" },
  { id: "background_08", label: "Monster Tier II · fixture", assetPath: "/content/releases/visible-world-v1/village/background/background_08__1530.png" },
  { id: "background_11", label: "Monster Tier III · fixture", assetPath: "/content/releases/visible-world-v1/village/background/background_11__1508.png" },
] as const;

export type MonsterMapId = (typeof MONSTER_MAP_FIXTURES)[number]["id"];
export type MonsterIntent = { type: "target_monster"; entityId: string; mapId: MonsterMapId };
export interface MonsterMapProjection { id: MonsterMapId; label: string; assetPath: string }

export interface MonsterFarmProjection extends MonsterMapProjection {
  densityLevel: number;
  spawnCount: number;
}

export interface MonsterSpawnProjection {
  current: number;
  minimum: number | null;
  maximum: number | null;
  evidenceState: "fixture_current_only" | "unresolved";
}

export interface MonsterRowProjection {
  entityId: string;
  family: "mon_a_01_1" | "mon_goldblin";
  state: "alive" | "dead" | "respawning";
  targetable: boolean;
  dropCount: number;
  sourceIndex: number | null;
  hp: number | null;
  maxHp: number | null;
  damage: number | null;
  armor: number | null;
  experience: number | null;
  gold: number | null;
}

export interface MonsterFieldProjection {
  fixtureLabel: string;
  selectedMap: MonsterMapId;
  maps: readonly MonsterMapProjection[];
  farms: readonly MonsterFarmProjection[];
  spawn: MonsterSpawnProjection;
  monsters: MonsterRowProjection[];
  respawnEvent: boolean;
  dropCount: number;
  densityLevel?: number;
  bannerMessage?: string | null;
}

export function projectMonsterField(
  entities: WorldEntityProjection[],
  combat: FixtureCombatWorldSnapshot | null,
  selectedMap: MonsterMapId = "map_new01",
): MonsterFieldProjection {
  const monsters = entities
    .filter((entity) => entity.descriptor.kind === "monster")
    .map((entity) => ({
      entityId: entity.descriptor.entity_id,
      family: monsterFamily(entity.descriptor.source_skeleton_name),
      state: entity.action_state === "idle" && entity.animation === "die" ? "dead" : "alive",
      targetable: false,
      dropCount: 0,
      sourceIndex: null,
      hp: entity.current_hp,
      maxHp: entity.maximum_hp,
      damage: null,
      armor: null,
      experience: null,
      gold: null,
    } satisfies MonsterRowProjection));
  const combatMonster = combat?.monster;
  const combatDead = combatMonster !== undefined && (!combatMonster.alive || combatMonster.state === "dead");
  if (combatMonster && monsters.length === 0) {
    monsters.push({
      entityId: `monster:${combatMonster.id}`,
      family: "mon_a_01_1",
      state: combatDead ? "dead" : "alive",
      targetable: false,
      dropCount: combat?.ground_drops.length ?? 0,
      sourceIndex: null,
      hp: combatMonster.hp,
      maxHp: combatMonster.max_hp,
      damage: null,
      armor: null,
      experience: null,
      gold: null,
    });
  }
  return {
    fixtureLabel: "visible-world monster presentation fixture; spawn semantics unresolved",
    selectedMap,
    maps: MONSTER_MAP_FIXTURES,
    farms: MONSTER_MAP_FIXTURES.map((map) => ({ ...map, densityLevel: 1, spawnCount: 0 })),
    spawn: {
      current: monsters.length,
      minimum: null,
      maximum: null,
      evidenceState: monsters.length > 0 ? "fixture_current_only" : "unresolved",
    },
    monsters,
    respawnEvent: combat?.events.some((event) => event.type === "monster_respawned") ?? false,
    dropCount: combat?.ground_drops.reduce((total, drop) => total + drop.quantity, 0) ?? monsters.reduce((total, monster) => total + monster.dropCount, 0),
  };
}

export function projectAuthoritativeMonsterField(world: MonsterWorldSnapshot): MonsterFieldProjection {
  const densityCounts = [
    [1, 4, 8],
    [2, 6, 10],
    [3, 7, 12],
  ] as const;
  const farms = world.maps.map((map, index) => ({
    id: map.map_id as MonsterMapId,
    label: `Farm ${romanNumber(map.monster_tier)}`,
    assetPath: map.map_asset_id,
    densityLevel: map.density_level,
    spawnCount: densityCounts[index]?.[map.density_level - 1] ?? 0,
  }));
  return {
    fixtureLabel: world.ruleset,
    selectedMap: (MONSTER_MAP_FIXTURES.some((map) => map.id === world.map_id) ? world.map_id : "map_new01") as MonsterMapId,
    maps: farms,
    farms,
    spawn: { current: world.spawn_count, minimum: world.spawn_min, maximum: world.spawn_max, evidenceState: "fixture_current_only" },
    monsters: world.monsters.map((monster) => ({
      entityId: monster.entity_id,
      family: monster.monster_id === "mon_goldblin" ? "mon_goldblin" : "mon_a_01_1",
      state: monster.respawn_ticks !== null || monster.hp === 0 ? "respawning" : "alive",
      targetable: false,
      dropCount: world.drops.filter((drop) => drop.monster_entity_id === monster.entity_id).reduce((sum, drop) => sum + drop.quantity, 0),
      sourceIndex: monster.source_index,
      hp: monster.hp,
      maxHp: monster.max_hp,
      damage: monster.damage,
      armor: monster.armor,
      experience: monster.experience,
      gold: monster.gold,
    })),
    respawnEvent: world.monsters.some((monster) => monster.respawn_ticks !== null),
    dropCount: world.drops.reduce((sum, drop) => sum + drop.quantity, 0),
    densityLevel: world.density_level,
    bannerMessage: world.banner_message,
  };
}

export function validateMonsterIntent(
  projection: MonsterFieldProjection,
  entityId: string,
  mapId: string,
): MonsterIntent | null {
  if (!projection.maps.some((map) => map.id === mapId)) return null;
  const monster = projection.monsters.find((candidate) => candidate.entityId === entityId);
  if (!monster || !monster.targetable || monster.state !== "alive") return null;
  return { type: "target_monster", entityId, mapId: mapId as MonsterMapId };
}

function monsterFamily(source: string): MonsterRowProjection["family"] {
  return source === "mon_goldblin" ? "mon_goldblin" : "mon_a_01_1";
}

function romanNumber(value: number): string {
  return ["I", "II", "III"][value - 1] ?? "?";
}
