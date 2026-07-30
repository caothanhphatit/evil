import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");

describe("static scene rendering", () => {
  it("bakes immutable town and farm scenery separately from dynamic entities", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/game/visible-world.ts"), "utf8");
    expect(source).toContain("private readonly staticLayer = new Container()");
    expect(source).toContain("this.addScenePieces(manifest.village.tiles, this.staticLayer)");
    expect(source).toContain("target.addChild(sprite)");
    expect(source).toContain("this.staticLayer.cacheAsTexture({ antialias: false })");
    expect(source).toContain("this.root.addChild(this.staticLayer, this.worldLayer)");
  });

  it("reports the measured Pixi ticker rate in the game HUD", async () => {
    const [source, shell] = await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/world-controller.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/shell.ts"), "utf8"),
    ]);
    expect(shell).toContain('id="fps-counter"');
    expect(source).toContain("Math.round(app.ticker.FPS)");
  });

  it("buffers one authoritative 10 Hz simulation tick for smooth world motion", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/game/visible-world.ts"), "utf8");
    expect(source).toContain("tickDurationMs: 100");
    expect(source).toContain("renderDelayMs: 100");
    expect(source).toContain("maxExtrapolationTicks: 1");
  });

  it("uses one camera zoom for the shared town and farm world", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/game/scene-projection.ts"), "utf8");
    expect(source).toContain("export const TOWN_CAMERA_ZOOM = 1.92");
    expect(source).toContain("export const FIELD_CAMERA_ZOOM = TOWN_CAMERA_ZOOM");
    expect(source).toContain("export const MIN_CAMERA_ZOOM = 1.44");
    expect(source).toContain("export const MAX_CAMERA_ZOOM = 2.2");
  });

  it("warms the first world frame before enabling entry", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/world-controller.ts"), "utf8");
    expect(source).toContain("app.render()");
    expect(source).toContain("context.entryController.markMapReady");
  });

  it("keeps a restored server session behind the login gate until the player enters", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/entry-controller.ts"), "utf8");
    expect(source).toContain('private phase: EntryPhase = "login"');
    expect(source).toContain("projectEntryPresentation(this.phase)");
    expect(source).toContain("const village = entry.renderWorld &&");
    expect(source).toContain('this.bottomMenu.hidden = !entry.enableGameUi || snapshot.screen === "boot"');
    expect(source).toContain('this.loginScreen.classList.toggle("leaving", !entry.showLogin)');
    expect(source).toContain("scheduleReveal");
    expect(source).toContain('this.enterVillage.disabled = this.phase !== "login" || this.mapLoadFailed');
    expect(source).toContain('if (this.mapLoadFailed || this.phase !== "login") return');
  });

  it("uses distinct login and full-screen loading presentations", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/shell.ts"), "utf8");
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    expect(source.indexOf('id="login-screen"')).toBeLessThan(source.indexOf('id="game-loading-screen"'));
    expect(source).toContain('class="basic-login-card"');
    expect(source).toContain('class="game-loading-content"');
    expect(styles).toContain("url('/content/loading/ashen-frontier-loading.png')");
    expect(styles).not.toContain(".game-loading-card");
  });

  it("defers server bootstrap and game assets until sign in", async () => {
    const source = await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/game-application.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/entry-controller.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/shell.ts"), "utf8"),
    ]).then((parts) => parts.join("\n"));
    expect(source.match(/client\.connect\(\)/g)).toHaveLength(1);
    expect(source).toContain("function startGameRuntime(): void");
    expect(source).toContain('document.querySelectorAll<HTMLImageElement>("img[data-game-src]")');
    expect(source).toContain("this.onStartRuntime();");
    expect(source).toContain('mountGameShell(mount)');
  });

  it("fails visibly instead of leaving loading parked at 92 percent", async () => {
    const [source, styles] = await Promise.all([
      Promise.all([
        readFile(resolve(repositoryRoot, "apps/web/src/app/entry-controller.ts"), "utf8"),
        readFile(resolve(repositoryRoot, "apps/web/src/app/shell.ts"), "utf8"),
      ]).then((parts) => parts.join("\n")),
      readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8"),
    ]);
    expect(source).toContain('id="game-loading-retry"');
    expect(source).toContain("}, 30_000)");
    expect(source).toContain('gameLoadingPercent.textContent = t("loading.error_title")');
    expect(source).toContain("gameLoadingRetry.hidden = false");
    expect(styles).toContain(".game-loading-content > button[hidden] { display: none; }");
  });
});
