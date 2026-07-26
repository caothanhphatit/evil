import type { HunterView } from "../hunter-roster";

export type HunterInfoTabId = "status" | "skills" | "growth" | "riding" | "materials";

export interface HunterInfoSkill {
  id: string;
  name: string;
  icon: string | null;
  level: number | null;
  description: string | null;
  group: string | null;
  unlocked: boolean | null;
  unlockRequirement: string | null;
}

export interface HunterInfoGrowthNode {
  id: string;
  icon: string | null;
  points: number;
  maxPoints: number;
  order: number;
}

export interface HunterInfoMaterial {
  id: string;
  name: string | null;
  icon: string;
  quantity: number;
  order: number;
}

export interface HunterInfoEquipmentSlot {
  id: string;
  icon: string | null;
  placeholderIcon: string | null;
  locked: boolean | null;
}

export interface HunterInfoView {
  hunter: HunterView;
  title: string;
  locked: boolean | null;
  reincarnation: { current: number; maximum: number } | null;
  experience: { current: number; maximum: number } | null;
  dps: number | null;
  criticalChance: number | null;
  attackSpeed: number | null;
  evasion: number | null;
  awakening: { current: number; maximum: number } | null;
  equipment: HunterInfoEquipmentSlot[];
  skills: HunterInfoSkill[] | null;
  growth: { secretPoints: number; nodes: HunterInfoGrowthNode[] } | null;
  riding: { mounted: false; canMoveToRanch: boolean } | { mounted: true; name: string | null; icon: string | null } | null;
  materials: HunterInfoMaterial[] | null;
}
