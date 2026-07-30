import { describe, expect, it } from "vitest";
import { tradeSettlementText } from "./visible-world";

describe("Hunter trade presentation", () => {
  it("shows authoritative gold and every sold material line", () => {
    expect(tradeSettlementText({
      trade_gold: 80,
      trade_materials: [
        { material_id: "material:32", display_name: "Vải Lanh", quantity: 2 },
        { material_id: "material:92", display_name: "Bột Ma Thuật", quantity: 3 },
      ],
    })).toBe("+80 Vàng\nĐã bán Vải Lanh x2 · Đã bán Bột Ma Thuật x3");
  });
});
