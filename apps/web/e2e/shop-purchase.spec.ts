import { expect, test, type Page } from "@playwright/test";

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto("/?e2e=1");
  await page.locator("#login-email").fill(email);
  await page.locator("#login-password").fill("Demo1234!");
  await page.locator("#enter-village").click();
  await expect(page.locator("#game-loading-screen")).toBeHidden({ timeout: 50_000 });
  await expect(page.locator("#village-screen")).toHaveClass(/visible/, { timeout: 50_000 });
}

test("display shop opens an item detail with authoritative stats and buyer selection", async ({ page }, testInfo) => {
  test.setTimeout(240_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const email = testInfo.project.name === "desktop-13" ? "demo3@evil.local" : "demo2@evil.local";
  await signIn(page, email);

  const opened = await page.evaluate(() => window.__EVIL_HUNTER_E2E__?.openBuilding("build_7") ?? false);
  expect(opened).toBe(true);
  const card = page.locator("#building-catalog .display-card").first();
  await expect(card).toBeVisible();
  await card.click();

  const popup = page.locator("#gear-create-pop");
  await expect(popup).toBeVisible();
  await expect(popup.locator(".shop-item-stats dd").first()).not.toHaveText("");
  await expect(popup.locator(".shop-buyer-list button")).not.toHaveCount(0);
  await expect(popup.locator("#gear-create-sell")).toBeDisabled();
  const buyer = popup.locator(".shop-buyer-list button:not(:disabled)").first();
  const hunterId = Number(await buyer.getAttribute("data-hunter-id"));
  const goldBefore = await page.evaluate((id) => window.__EVIL_HUNTER_E2E__?.snapshot()?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === id)?.gold ?? null, hunterId);
  await buyer.click();
  await expect(popup.locator("#gear-create-sell")).toBeEnabled();
  await popup.locator("#gear-create-sell").click();
  await expect(popup).toBeHidden({ timeout: 90_000 });
  await expect.poll(async () => page.evaluate((id) => window.__EVIL_HUNTER_E2E__?.snapshot()?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === id)?.gold ?? null, hunterId), { timeout: 30_000 }).toBeLessThan(goldBefore ?? 0);
  await expect(page.locator("#building-panel")).toBeVisible();

  await card.click();
  await expect(popup).toBeVisible();
  await expect(popup.locator(".shop-item-stats dd").first()).not.toHaveText("");
  const fitsViewport = await popup.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    return bounds.top >= 0 && bounds.left >= 0 && bounds.right <= window.innerWidth && bounds.bottom <= window.innerHeight;
  });
  expect(fitsViewport).toBe(true);
  expect(pageErrors).toEqual([]);
  await page.screenshot({ path: `/tmp/shop-purchase-${testInfo.project.name}.png`, fullPage: true });
});
