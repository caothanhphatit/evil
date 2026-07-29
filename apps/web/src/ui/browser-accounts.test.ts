import { describe, expect, it } from "vitest";
import { DEFAULT_DEMO_ACCOUNT, parseBrowserAccounts, serializeBrowserAccounts } from "./browser-accounts";

describe("browser account profiles", () => {
  it("always provides a disposable demo account", () => {
    expect(parseBrowserAccounts(null)).toEqual([DEFAULT_DEMO_ACCOUNT]);
    expect(parseBrowserAccounts("not-json")).toEqual([DEFAULT_DEMO_ACCOUNT]);
  });

  it("round-trips only the supported browser profile shape", () => {
    const accounts = [DEFAULT_DEMO_ACCOUNT, { id: "a", displayName: "A", email: "a@example.test", kind: "browser" as const, createdAt: "now" }];
    expect(parseBrowserAccounts(serializeBrowserAccounts(accounts))).toEqual(accounts);
    expect(parseBrowserAccounts(JSON.stringify([{ id: "bad" }, ...accounts]))).toEqual(accounts);
  });
});
