import { expect, test } from "@playwright/test";

test("sign in, load the authoritative town, and open the Hunter roster", async ({ page }) => {
  const telemetry: unknown[] = [];
  await page.exposeFunction("captureTelemetry", (event: unknown) => telemetry.push(event));
  await page.addInitScript(() => {
    window.addEventListener("evil:telemetry", (event) => {
      void (window as unknown as { captureTelemetry: (detail: unknown) => Promise<void> }).captureTelemetry((event as CustomEvent).detail);
    });
  });

  await page.goto("/");
  await expect(page.locator("#login-screen")).toBeVisible();
  await expect(page.locator("#game-loading-screen")).toBeHidden();
  await page.locator("#enter-village").click();
  await expect(page.locator("#game-loading-screen")).toBeVisible();
  await expect(page.locator("#village-screen")).toHaveClass(/visible/, { timeout: 50_000 });
  await expect(page.locator("#bottom-menu")).toBeVisible();

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
