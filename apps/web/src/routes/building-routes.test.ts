import { describe, expect, it } from "vitest";
import { BOUNTY_HUT_ROUTE, BOUNTY_TIERS, bountyHutView } from "./bounty-hut";
import { TRADING_POST_ROUTE, tradingPostDifficultyOptions, tradingPostStocksForDifficulty, tradingPostView } from "./trading-post";

describe("decoded building route contracts", () => {
  it("keeps Trading Post on BuildingPop and exposes reservation state", () => {
    expect(TRADING_POST_ROUTE.popup).toEqual({ template: "BuildingPop", width: 560, height: 900 });
    const view = tradingPostView({ level: 1, townGold: 10000, difficulty: 0, stocks: [{ id: "material:1", displayName: "Linen Cloth", icon: null, townQuantity: 0, hunterQuantity: 2, requested: 50, unitPrice: 10, difficulty: 0 }] });
    expect(view.requestCount).toBe(1);
    expect(view.stocks[0].remainingRequest).toBe("50");
    expect(view.stocks[0].requestLabel).toBe("Cancel");
  });

  it("uses QuestPop for Bounty Hut with screenshot tiers", () => {
    expect(BOUNTY_HUT_ROUTE.popup).toEqual({ template: "QuestPop", width: 480, height: 820 });
    expect(BOUNTY_TIERS.map((tier) => tier.kills)).toEqual([15, 45, 135, 405]);
    expect(bountyHutView({ level: 1, tier: 2, quests: [] }).tierLabel).toBe("Large");
  });

  it("never fabricates unresolved item icons", () => {
    const view = tradingPostView({ level: 1, townGold: 0, difficulty: 0, stocks: [{ id: "material:unknown", displayName: "Unknown", icon: null, townQuantity: 0, hunterQuantity: 0, requested: 0, unitPrice: 0, difficulty: 0 }] });
    expect(view.stocks[0].icon).toBeNull();
  });

  it("unlocks decoded material ratings by Trading Post level and keeps later modes locked", () => {
    expect(tradingPostDifficultyOptions(2).map((tab) => tab.unlocked)).toEqual([
      true, true, false, false, false, false, false, false, false,
    ]);
    expect(tradingPostStocksForDifficulty([
      { id: "easy", difficulty: 0 },
      { id: "normal", difficulty: 1 },
      { id: "easy-2", difficulty: 0 },
    ], 0).map((stock) => stock.id)).toEqual(["easy", "easy-2"]);
  });
});
