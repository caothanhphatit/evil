export interface GroundDrop { drop_id: string; item_id: number; quantity: number; x: number; y: number }
export interface InventoryStack { item_id: number; quantity: number }
export type LegacyEntityState = "idle" | "moving" | "attacking" | "dead" | "reviving";
export interface ServerEntitySnapshot { id: number; hp: number; max_hp: number; alive: boolean; x: number; y: number; state: LegacyEntityState }

export type EntityKind = "hunter" | "monster";
export type Facing = "front" | "back";

export interface EntityState extends ServerEntitySnapshot {
  kind: EntityKind;
  name: string;
  facing: Facing;
}

export interface WorldSnapshot {
  sequence: number;
  serverTime: number;
  fighting: boolean;
  gold: number;
  entities: EntityState[];
  inventory: InventoryStack[];
  equippedItemId: number | null;
  groundDrops: GroundDrop[];
}

export interface ContentAsset {
  id: string;
  unitId: string;
  publicPath: string;
}

export interface ContentUnit {
  id: string;
  kind: string;
  status?: string;
  assetIds: string[];
}

export interface ContentRelease {
  releaseId: string;
  units: Map<string, ContentUnit>;
  assets: Map<string, ContentAsset>;
}
