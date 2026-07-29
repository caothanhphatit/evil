import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { equipmentDetailText } from "./modal";

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
    const main = await readFile(resolve(repositoryRoot, "apps/web/src/main.ts"), "utf8");
    expect(styles).toContain("inset: 0 0 calc(var(--bottom-menu-bottom) + var(--bottom-menu-reserved))");
    expect(styles).toContain("width: min(88%, 440px)");
    expect(styles).toContain("grid-template-rows: auto minmax(230px, 46%) auto minmax(150px, 1fr) auto");
    expect(styles).toContain("height: min(94%, 690px)");
    expect(styles).toContain(".hunter-info-actor-canvas");
    expect(styles).toContain("button:focus { outline: 0; }");
    expect(styles).toContain("button:focus-visible { outline: 3px solid #f7cf62");
    expect(modal).toContain("panel.focus({ preventScroll: true })");
    expect(main).toContain("info.blur()");
  });

  it("keeps equipment controls balanced inside the persistent-menu safe area", async () => {
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    const modal = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    expect(styles).toContain("grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(78px, 1.45fr) minmax(0, 1fr) minmax(0, 1fr)");
    expect(styles).toContain("width: min(100%, 356px)");
    expect(styles).toContain("grid-template-rows: repeat(3, minmax(0, 1fr))");
    expect(styles).toContain("width: 100%; min-width: 0; padding: 0; aspect-ratio: 1");
    expect(styles).toContain("width: min(100%, 286px)");
    expect(styles).toContain("width: min(100%, 226px)");
    expect(styles).toContain("min-height: 38px");
    expect(styles).toContain("hunter-info-equipment/equip_bg_9__2684.png");
    expect(styles).not.toMatch(/\.hunter-equipment-slot \{[^}]*box_gear_9__2514\.png/s);
    expect(modal).toContain('boots: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_06__1917.png"');
    expect(modal).toContain('armor: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_04__5943.png"');
    expect(modal).toContain('slot.dataset.slotId = slotId');
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
    expect(source).toContain("EXP unavailable");
    expect(source).not.toContain("Look unavailable");
    expect(actor).toContain('const animation = "hunter_stay"');
    expect(actor).not.toContain('visual.animation ?? "hunter_stay"');
    expect(actor).toContain("height * 0.74 / bounds.height");
    expect(actor).toContain("2.4,");
    expect(rosterActors).toContain('const animation = "hunter_stay"');
    expect(rosterActors).not.toContain('visual.animation ?? "hunter_stay"');
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
    expect(styles).toContain(".hunter-roster-card.selected { border-color: #948668; box-shadow: none; }");
    expect(styles).toContain(".hunter-card-info:focus, .hunter-card-info:focus-visible { outline: 0; }");
    expect(styles).not.toContain("border-image: url('/content/releases/original-flow-v1/sprites/popup_bg_9__1928.png') 7 fill");
  });

  it("keeps unresolved Hunter tabs framed instead of collapsing to plain text", async () => {
    const materials = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/materials-tab.ts"), "utf8");
    const growth = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/growth-tab.ts"), "utf8");
    const riding = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/riding-pet-tab.ts"), "utf8");
    expect(materials).toContain("UNRESOLVED_PLACEHOLDER_COUNT = 12");
    expect(growth).toContain("index < 15");
    expect(riding).toContain("Move to Ranch");
  });

  it("presents carried loot as the original Material section", async () => {
    const modal = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/modal.ts"), "utf8");
    const inventory = await readFile(resolve(repositoryRoot, "apps/web/src/ui/hunter-info/materials-tab.ts"), "utf8");
    expect(modal).toContain('{ id: "materials", label: "Material" }');
    expect(inventory).toContain('node("h3", "", "Material")');
    expect(inventory).not.toContain("INVENTORY_SLOT_COUNT");
  });
});
