import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { calculateExperiencePercent, calculateHunterInfoScale, equipmentDetailText } from "./modal";

const repositoryRoot = resolve(import.meta.dirname, "../../../../..");

describe("Hunter Info modal shell", () => {
  it("swaps only the tab body so the source frame remains mounted", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    expect(source).toContain('querySelector<HTMLElement>(".hunter-info-tab-body")');
    expect(source).toContain("body.replaceChildren(renderTab(activeTab, info, actions))");
    expect(source).not.toContain("activeTab = tab; render();");
  });

  it("keeps a content-sized frame and clears the roster trigger focus", async () => {
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    const modal = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    const main = (await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/game-application.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/hunter-controller.ts"), "utf8"),
    ])).join("\n");
    expect(styles).toContain("inset: 0 0 calc(var(--bottom-menu-bottom) + var(--bottom-menu-reserved))");
    expect(styles).toContain("width: 440px");
    expect(styles).toContain("grid-template-rows: auto minmax(230px, 46%) auto minmax(150px, 1fr) auto");
    expect(styles).toContain("height: 690px");
    expect(styles).toContain("transform-origin: top left");
    expect(styles).toContain("popup_bg_9__1928.png') 20 fill / 18px stretch");
    expect(modal).toContain('node("div", "hunter-info-scale-frame")');
    expect(modal).toContain("new ResizeObserver(syncScale).observe(overlay)");
    expect(styles).toContain(".hunter-info-actor-canvas");
    expect(styles).toContain("button:focus { outline: 0; }");
    expect(styles).toContain("button:focus-visible { outline: 3px solid #f7cf62");
    expect(modal).toContain("panel.focus({ preventScroll: true })");
    expect(main).toContain("trigger.blur()");
  });

  it("keeps equipment controls balanced inside the persistent-menu safe area", async () => {
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    const modal = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    expect(styles).toContain("grid-template-columns: 42px 42px minmax(112px, 1fr) 42px 42px");
    expect(styles).toContain("width: min(100%, 330px)");
    expect(styles).toContain("grid-template-rows: repeat(3, minmax(0, 1fr))");
    expect(styles).toContain("width: 42px; min-width: 0; justify-self: center; padding: 0; aspect-ratio: 1");
    expect(styles).toContain("width: 36px; min-width: 0; justify-self: center; aspect-ratio: 1");
    expect(styles).toContain("min-height: 38px");
    expect(styles).toContain("hunter-info-equipment/equip_bg_9__2684.png");
    expect(styles).not.toMatch(/\.hunter-equipment-slot \{[^}]*box_gear_9__2514\.png/s);
    expect(modal).toContain('boots: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_06__1917.png"');
    expect(modal).toContain('armor: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_04__5943.png"');
    expect(modal).toContain('slot.dataset.slotId = slotId');
  });

  it("scales the complete design canvas by its tightest viewport boundary", () => {
    expect(calculateHunterInfoScale(440, 690)).toBe(1);
    expect(calculateHunterInfoScale(320, 690)).toBeCloseTo(320 / 440);
    expect(calculateHunterInfoScale(440, 345)).toBe(.5);
    expect(calculateHunterInfoScale(880, 1380)).toBe(1);
  });

  it("keeps Hunter EXP progress bounded by the authoritative current and maximum values", async () => {
    expect(calculateExperiencePercent(262, 322)).toBeCloseTo(81.366);
    expect(calculateExperiencePercent(-4, 322)).toBe(0);
    expect(calculateExperiencePercent(400, 322)).toBe(100);
    expect(calculateExperiencePercent(10, 0)).toBe(0);

    const source = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    expect(source).toContain('track.setAttribute("role", "progressbar")');
    expect(source).toContain('t("hunter.info.exp", { current: info.experience.current, maximum: info.experience.maximum })');
    expect(styles).toContain(".hunter-experience { position: relative; height: 22px; margin: 2px 2px 0; }");
    expect(styles).toContain("background: linear-gradient(90deg, #6f069e, #a915d4)");
    expect(styles).not.toContain("exp_gauge_in_9__6967.png");
    expect(styles).toContain("font-size: 9px");
  });

  it("keeps server-authoritative hunt commands outside the original Detail popup", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    expect(source).not.toContain('action("Assign Zone"');
    expect(source).not.toContain("hunter-info-command-panel");
    expect(source).not.toContain("advanceHunterHunt");
  });

  it("renders the recovered Hunter actor and all eight confirmed equipment positions", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    const actor = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/actor.ts"), "utf8");
    const rosterActors = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-roster-actors.ts"), "utf8");
    const presentationAssets = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-presentation-assets.ts"), "utf8");
    expect(source).toContain("createHunterInfoActor");
    expect(source).toContain("UTILITY_SLOT_COUNT = 6");
    expect(source).toContain('left: ["ring", "weapon", "necklace"]');
    expect(source).toContain('right: ["armor", "gloves", "boots"]');
    expect(source).toContain('node("div", "hunter-center-loadout")');
    expect(source).toContain('equipment.get("helmet")');
    expect(source).toContain('equipment.get("belt")');
    expect(source).toContain("equip_dummy_08__5673.png");
    expect(source).toContain("hunter-utility-slot");
    expect(source).toContain("EQUIPMENT_PLACEHOLDERS");
    expect(source).toContain('slot.addEventListener("click", () => select(equipment))');
    expect(source).toContain("hunter-equipment-detail");
    expect(source).toContain('t("hunter.info.exp_unavailable")');
    expect(source).not.toContain("Look unavailable");
    expect(actor).toContain('const animation = "hunter_stay"');
    expect(actor).not.toContain('visual.animation ?? "hunter_stay"');
    expect(actor).toContain("height * 0.74 / bounds.height");
    expect(actor).toContain("2.4,");
    expect(rosterActors).toContain('const animation = "hunter_stay"');
    expect(rosterActors).not.toContain('visual.animation ?? "hunter_stay"');
    expect(rosterActors).toContain("avatarBounds.height * 0.9");
    expect(rosterActors).toContain("spine.mask = clip");
    expect(actor).toContain("preload: initialize");
    expect(rosterActors).toContain("preloadHunterPresentationAssets()");
    expect(presentationAssets).toContain('HUNTER_SKELETON_ALIAS = "hunter:presentation:skeleton"');
    expect(presentationAssets).toContain("let preloadPromise: Promise<void> | null = null");
  });

  it("preloads both Hunter Info canvases before the game loading screen completes", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/world-controller.ts"), "utf8");
    const modal = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    expect(modal).toContain("preload: () => actor.preload()");
    expect(source).toContain("hunterInfoModal.preload()");
    expect(source).toContain("worldHunterInfoModal.preload()");
  });

  it("replaces the embedded Korean difficulty wording with the localized runtime label", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/shell.ts"), "utf8");
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    expect(source).toContain('<b id="world-mode-label">${t("world.easy")}</b>');
    expect(source).not.toContain('id="world-mode-label" class="sr-only"');
    expect(styles).toContain("#world-mode-label { position: absolute;");
    expect(styles).toContain("background: #08723e");
  });

  it("shows only evidence-backed equipment identity in the clickable item detail", () => {
    expect(equipmentDetailText({
      id: "weapon",
      catalogKind: "weapon",
      catalogIndex: 9,
      name: "Junk Hammer",
      icon: null,
      placeholderIcon: null,
      presentationGender: "male",
      requiredClassId: "h2",
      locked: false,
      evidenceState: "migration-fixture",
    })).toBe("weapon #9 · Evidence: migration-fixture");
  });

  it("can close an unopened Hunter modal without touching an uninitialized Pixi canvas", async () => {
    const actor = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/actor.ts"), "utf8");
    expect(actor).toContain("let ready = false");
    expect(actor).toContain("if (!ready) return");
    expect(actor).toContain("epoch !== renderEpoch");
  });

  it("does not render the roster selection as a focus frame", async () => {
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    expect(styles).toContain(".hunter-roster-card.selected { border-color: transparent; filter: brightness(1.08)");
    expect(styles).toContain(".hunter-card-info:focus, .hunter-card-info:focus-visible { outline: 0; }");
    expect(styles).not.toContain("border-image: url('/content/releases/original-flow-v1/sprites/popup_bg_9__1928.png') 7 fill");
  });

  it("keeps the desktop roster compact and scrollable beyond two rows", async () => {
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    const main = await readFile(resolve(repositoryRoot, "apps/web/src/app/game-application.ts"), "utf8");
    const actors = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-roster-actors.ts"), "utf8");
    expect(styles).toContain("width: min(820px, calc(100% - 32px))");
    expect(styles).toContain("grid-auto-rows: 214px");
    expect(styles).toContain("overflow-y: auto");
    expect(styles).toContain("height: 214px");
    expect(main).toContain('hunterActiveList.addEventListener("scroll"');
    expect(main).toContain("hunterRosterActors.refresh()");
    expect(actors).toContain("refresh(): void");
  });

  it("keeps unresolved Hunter tabs framed instead of collapsing to plain text", async () => {
    const materials = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/materials-tab.ts"), "utf8");
    const growth = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/growth-tab.ts"), "utf8");
    const riding = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/riding-pet-tab.ts"), "utf8");
    expect(materials).toContain("UNRESOLVED_PLACEHOLDER_COUNT = 12");
    expect(growth).toContain("index < 15");
    expect(riding).toContain('t("hunter.riding.move_ranch")');
  });

  it("presents carried loot as the original Material section", async () => {
    const modal = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    const inventory = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/materials-tab.ts"), "utf8");
    expect(modal).toContain('{ id: "materials", label: t("hunter.tabs.materials") }');
    expect(inventory).toContain('node("h3", "", t("hunter.materials.title"))');
    expect(inventory).not.toContain("INVENTORY_SLOT_COUNT");
  });
});
