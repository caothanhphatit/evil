import { t } from "../i18n";

/** Route contract recovered from BuildingPop (build_3), not RequestPop. */
export const TRADING_POST_ROUTE = {
  buildingId: "build_3",
  routeId: "trading-post-purchase",
  popup: { template: "BuildingPop", width: 560, height: 900 },
  title: t("trading.title"),
  description: t("trading.description"),
  widgets: ["TextTab", "ratingTab", "MoneyChange", "CreatePossible", "RequestStateButton", "GridBorder", "GridSecondBorder"],
  tabs: [t("difficulty.easy"), t("difficulty.normal"), t("difficulty.hard"), t("difficulty.expert"), t("difficulty.nightmare"), t("difficulty.torment")],
  upgrade: {
    maxLevel: 6,
    gold: [300, 4800, 16200, 48600, 145800, 437400],
    effect: "Adds purchase reservation list of [difficulty] difficulty",
    townHallLevel: 5,
  },
  assets: {
    popup: "/content/releases/original-flow-v1/sprites/popup_bg_9__1928.png",
    close: "/content/releases/original-flow-v1/sprites/btn_red_01_9__3691.png",
    upgrade: "/content/releases/original-flow-v1/sprites/btn_green_01_9__5553.png",
    gold: "/content/releases/original-flow-v1/sprites/top_ic_01_gold_24__4677.png",
  },
} as const;

export interface TradingPostStock {
  id: string;
  displayName: string;
  icon: string | null;
  townQuantity: number;
  hunterQuantity: number;
  requested: number;
  unitPrice: number;
  difficulty: number;
}

export interface TradingPostState { level: number; townGold: number; difficulty: number; stocks: TradingPostStock[]; }

export function tradingPostView(state: TradingPostState) {
  const difficulty = Math.max(0, Math.min(TRADING_POST_ROUTE.tabs.length - 1, state.difficulty));
  return {
    ...TRADING_POST_ROUTE,
    level: state.level,
    difficulty,
    difficultyLabel: TRADING_POST_ROUTE.tabs[difficulty],
    requestCount: state.stocks.filter((stock) => stock.requested > 0).length,
    stocks: state.stocks.map((stock) => ({
      ...stock,
      remainingRequest: stock.requested > 0 ? String(stock.requested) : "",
      requestLabel: stock.requested > 0 ? t("trading.cancel_request") : t("common.request"),
    })),
  };
}

export const TRADING_POST_RATING_TABS = [
  t("difficulty.easy"), t("difficulty.normal"), t("difficulty.hard"), t("difficulty.expert"), t("difficulty.nightmare"), t("difficulty.torment_boost"),
  t("difficulty.super"), t("difficulty.chaos"), t("difficulty.abyss"),
] as const;

export function tradingPostDifficultyOptions(level: number) {
  return TRADING_POST_RATING_TABS.map((label, difficulty) => ({
    label,
    difficulty,
    unlocked: difficulty < TRADING_POST_ROUTE.tabs.length && difficulty < level,
  }));
}

export function tradingPostStocksForDifficulty<T extends { difficulty: number }>(
  stocks: readonly T[],
  difficulty: number,
): T[] {
  return stocks.filter((stock) => stock.difficulty === difficulty);
}
