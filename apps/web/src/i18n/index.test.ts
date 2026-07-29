import { describe, expect, it } from "vitest";
import { currentLocale, formatNumber, setLocale, t } from ".";

describe("localization runtime", () => {
  it("uses Vietnamese as the default and fallback locale", () => {
    expect(currentLocale()).toBe("vi");
    expect(t("common.close")).toBe("Đóng");
    expect(setLocale("en-US")).toBe("vi");
    expect(t("common.close")).toBe("Đóng");
  });

  it("substitutes named parameters without hiding unresolved placeholders", () => {
    expect(t("common.level", { level: 12 })).toBe("Cấp 12");
    expect(t("common.level")).toBe("Cấp {level}");
  });

  it("formats numbers with the active locale", () => {
    setLocale("vi");
    expect(formatNumber(1234567)).toBe(new Intl.NumberFormat("vi").format(1234567));
  });
});
