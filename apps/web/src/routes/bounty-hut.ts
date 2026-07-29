import { t } from "../i18n";

/** Route contract recovered from QuestPop (build_4). */
export const BOUNTY_HUT_ROUTE = {
  buildingId: "build_4",
  routeId: "bounty-quest-list",
  popup: { template: "QuestPop", width: 480, height: 820 },
  title: t("bounty.title"),
  description: t("bounty.description"),
  tabs: [t("bounty.tier.small"), t("bounty.tier.medium"), t("bounty.tier.large"), t("bounty.tier.grand")],
  widgets: ["difficulty tabs", "quest list", "monster frame", "reward frame", "UpgradeExplan", "UpgradeButton", "CloseButton"],
  upgrade: { maxLevel: 6, gold: [780, 6240, 21060, 63180, 189540, 568620], effect: "Adds bounty quest list of [difficulty] difficulty", townHallLevel: 5 },
  assets: {
    popup: "/content/releases/original-flow-v1/sprites/popup_bg_9__1928.png",
    close: "/content/releases/original-flow-v1/sprites/btn_red_01_9__3691.png",
    upgrade: "/content/releases/original-flow-v1/sprites/btn_green_01_9__5553.png",
    gold: "/content/releases/original-flow-v1/sprites/top_ic_01_gold_24__4677.png",
  },
} as const;

export interface BountyQuest { monsterId: string; monsterName: string; monsterIcon: string | null; reward: number; }
export interface BountyHutState { level: number; tier: number; quests: BountyQuest[]; }

const tierConfig = [
  { label: t("bounty.tier.small"), kills: 15, multiplier: 2 },
  { label: t("bounty.tier.medium"), kills: 45, multiplier: 1.8 },
  { label: t("bounty.tier.large"), kills: 135, multiplier: 1.6 },
  { label: t("bounty.tier.grand"), kills: 405, multiplier: 1.4 },
] as const;

export function bountyHutView(state: BountyHutState) {
  const tier = Math.max(0, Math.min(tierConfig.length - 1, state.tier));
  const config = tierConfig[tier];
  return { ...BOUNTY_HUT_ROUTE, level: state.level, tier, tierLabel: config.label, kills: config.kills, multiplier: config.multiplier, quests: state.quests };
}

export { tierConfig as BOUNTY_TIERS };
