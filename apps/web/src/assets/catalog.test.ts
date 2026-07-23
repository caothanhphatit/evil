import { describe, expect, it } from "vitest";
import { matchesSha256 } from "./catalog";

describe("content manifest integrity", () => {
  it("accepts the exact published bytes and rejects modified content", async () => {
    const digest = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    await expect(matchesSha256("hello", digest)).resolves.toBe(true);
    await expect(matchesSha256("hello!", digest)).resolves.toBe(false);
  });
});
