import { expect, test } from "@playwright/test";

test("sign in, load the authoritative town, and use semantic game hooks", async ({ page }) => {
  test.setTimeout(120_000);
  const telemetry: unknown[] = [];
  await page.exposeFunction("captureTelemetry", (event: unknown) => telemetry.push(event));
  await page.addInitScript(() => {
    window.addEventListener("evil:telemetry", (event) => {
      void (window as unknown as { captureTelemetry: (detail: unknown) => Promise<void> }).captureTelemetry((event as CustomEvent).detail);
    });
  });

  await page.goto("/?e2e=1");
  await expect(page.locator("#login-screen")).toBeVisible();
  await expect(page.locator("#game-loading-screen")).toBeHidden();
  await page.locator("#login-email").fill("demo3@evil.local");
  await page.locator("#login-password").fill("Demo1234!");
  await page.locator("#enter-village").click();
  await expect(page.locator("#game-loading-screen")).toBeVisible();
  await expect(page.locator("#game-loading-screen")).toBeHidden({ timeout: 50_000 });
  await expect(page.locator("#village-screen")).toHaveClass(/visible/, { timeout: 50_000 });
  await expect(page.locator("#bottom-menu")).toBeVisible();

  const buildingOpened = await page.evaluate(() => window.__EVIL_HUNTER_E2E__?.openBuilding("build_10") ?? false);
  expect(buildingOpened).toBe(true);
  await expect(page.locator("#building-panel")).toBeVisible();
  await page.locator("#building-panel-close").click();

  const hunterOpened = await page.evaluate(() => {
    const snapshot = window.__EVIL_HUNTER_E2E__?.snapshot();
    const hunterId = snapshot?.hunter_roster.active_hunters[0]?.hunter_id;
    return hunterId === undefined ? false : window.__EVIL_HUNTER_E2E__?.openHunterInfo(hunterId) ?? false;
  });
  expect(hunterOpened).toBe(true);
  await expect(page.locator(".hunter-info-overlay:not([hidden])")).toBeVisible();
  await page.locator(".hunter-info-close").click();

  await page.locator('#bottom-menu [data-action="character"]').click();
  await expect(page.locator("#roster-screen")).toHaveClass(/visible/);
  await expect(page.locator("#hunter-active-list .hunter-roster-card").first()).toBeVisible();
  await page.locator("#roster-back").click();
  await expect(page.locator("#roster-screen")).not.toHaveClass(/visible/);
  await expect(page.locator("#roster-screen")).toHaveAttribute("aria-hidden", "true");

  await page.locator('#bottom-menu [data-action="character"]').click();
  await expect(page.locator("#roster-screen")).toHaveClass(/visible/);

  const viewportFits = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1);
  expect(viewportFits).toBe(true);
  expect(telemetry).not.toContainEqual(expect.objectContaining({ level: "error" }));
});
