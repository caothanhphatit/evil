import { expect, test, type Page } from "@playwright/test";

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto("/?e2e=1");
  await page.locator("#login-email").fill(email);
  await page.locator("#login-password").fill("Demo1234!");
  await page.locator("#enter-village").click();
  await expect(page.locator("#game-loading-screen")).toBeHidden({ timeout: 50_000 });
  await expect(page.locator("#village-screen")).toHaveClass(/visible/, { timeout: 50_000 });
}

test("display shop keeps its catalog intact and buys for the preselected Hunter", async ({ page }, testInfo) => {
  test.setTimeout(240_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const email = testInfo.project.name === "desktop-13" ? "demo3@evil.local" : "demo2@evil.local";
  await signIn(page, email);

  const hunterId = await page.evaluate(() => window.__EVIL_HUNTER_E2E__?.snapshot()?.hunter_roster.active_hunters[0]?.hunter_id ?? null);
  expect(hunterId).not.toBeNull();
  const opened = await page.evaluate((id) => window.__EVIL_HUNTER_E2E__?.openHunterShop(id, "build_7") ?? false, hunterId!);
  expect(opened).toBe(true);
  const card = page.locator("#building-catalog .display-card").first();
  await expect(card).toBeVisible();
  await card.click();

  const popup = page.locator("#gear-create-pop");
  await expect(popup).toBeVisible();
  await expect(page.locator("#building-panel")).toBeVisible();
  await expect(popup.locator(".shop-item-stats dd").first()).not.toHaveText("");
  await expect(popup.locator(".shop-item-stats dd")).toHaveCount(4);
  await expect(popup.locator(".shop-weapon-comparison .current")).toBeVisible();
  await expect(popup.locator(".shop-weapon-comparison .candidate")).toBeVisible();
  await expect(popup.locator(".shop-weapon-comparison .candidate strong")).toHaveText(await card.locator("strong").innerText());
  await expect(popup.locator(".shop-purchase-economy dd")).toHaveCount(2);
  await expect(popup.locator(`.shop-buyer-gold[data-hunter-id="${hunterId}"]`)).toBeVisible();
  await expect(popup.locator(".shop-selected-buyer, .shop-buyer-list")).toHaveCount(0);
  const goldBefore = await page.evaluate((id) => window.__EVIL_HUNTER_E2E__?.snapshot()?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === id)?.gold ?? null, hunterId!);
  await expect(popup.locator("#gear-create-sell")).toBeEnabled();
  await popup.locator("#gear-create-sell").click();
  await expect(popup).toBeHidden({ timeout: 90_000 });
  await expect.poll(async () => page.evaluate((id) => window.__EVIL_HUNTER_E2E__?.snapshot()?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === id)?.gold ?? null, hunterId!), { timeout: 30_000 }).toBeLessThan(goldBefore ?? 0);
  await expect(page.locator("#building-panel")).toBeVisible();

  await card.click();
  await expect(popup).toBeVisible();
  await expect(popup.locator(".shop-item-stats dd").first()).not.toHaveText("");
  const fitsViewport = await popup.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    return bounds.top >= 0 && bounds.left >= 0 && bounds.right <= window.innerWidth && bounds.bottom <= window.innerHeight;
  });
  expect(fitsViewport).toBe(true);
  const popupAboveMenu = await popup.evaluate((element) => {
    const menu = document.querySelector<HTMLElement>(".bottom-menu");
    return menu === null || Number(getComputedStyle(element).zIndex) > Number(getComputedStyle(menu).zIndex);
  });
  expect(popupAboveMenu).toBe(true);
  expect(pageErrors).toEqual([]);
  await page.screenshot({ path: `/tmp/shop-purchase-${testInfo.project.name}.png`, fullPage: true });
});
