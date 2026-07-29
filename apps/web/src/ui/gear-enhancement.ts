import type { GearEnhancementSnapshot } from "../generated/protocol";

export const GEAR_ENHANCEMENT_MODES = ["single", "to_10", "to_15", "to_20"] as const;
export type GearEnhancementMode = typeof GEAR_ENHANCEMENT_MODES[number];
export type GearEnhancementView = "select" | "configure" | "processing" | "result";

export interface GearEnhancementPreview {
  productId: string;
  currentLevel: number | null;
  maxLevel: number;
  targetLevel: number | null;
  mode: GearEnhancementMode;
  costResolved: false;
  available: false;
  blockers: readonly string[];
}

export function projectGearEnhancement(
  row: GearEnhancementSnapshot,
  mode: GearEnhancementMode,
): GearEnhancementPreview {
  return {
    productId: row.product_id,
    currentLevel: row.level,
    maxLevel: row.max_level,
    targetLevel: enhancementTargetLevel(row.level, row.max_level, mode),
    mode,
    costResolved: false,
    available: false,
    blockers: [
      "enhancement_cost_binding",
      "enhancement_probability_binding",
      "enhancement_material_binding",
    ],
  };
}

export function enhancementTargetLevel(
  currentLevel: number | null,
  maxLevel: number,
  mode: GearEnhancementMode,
): number | null {
  if (currentLevel === null) return null;
  if (mode === "single") return Math.min(currentLevel + 1, maxLevel);
  const requested = mode === "to_10" ? 10 : mode === "to_15" ? 15 : 20;
  return Math.min(Math.max(currentLevel, requested), maxLevel);
}

export function canSubmitGearEnhancement(preview: GearEnhancementPreview): boolean {
  return preview.available && preview.costResolved;
}
