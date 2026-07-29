import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const sourceRoot = resolve(import.meta.dirname, "..");
const productionUiFiles = [
  "main.ts",
  "ui/panel-message.ts",
  "ui/hunter-roster.ts",
  "ui/hunter-world-command.ts",
  "ui/combat-hud.ts",
  "ui/monster-field.ts",
  "ui/hunter-info/modal.ts",
  "ui/hunter-info/status-tab.ts",
  "ui/hunter-info/skills-tab.ts",
  "ui/hunter-info/materials-tab.ts",
  "ui/hunter-info/growth-tab.ts",
  "ui/hunter-info/riding-pet-tab.ts",
  "routes/bounty-hut.ts",
  "routes/trading-post.ts",
  "content/building-evidence.ts",
  "content/blacksmith-route.ts",
  "content/product-service-routes.ts",
];

const sinkPatterns = [
  /(?:textContent|title)\s*=\s*(["'`])([^\n]*?)\1/g,
  /setAttribute\(\s*["'](?:aria-label|title)["']\s*,\s*(["'`])([^\n]*?)\1/g,
  /showPanelMessage\(\s*(["'`])([^\n]*?)\1/g,
];

describe("localized player-facing wording", () => {
  it("does not put hard-coded words directly into UI sinks", async () => {
    const violations: string[] = [];
    for (const relativePath of productionUiFiles) {
      const source = await readFile(resolve(sourceRoot, relativePath), "utf8");
      for (const pattern of sinkPatterns) {
        for (const match of source.matchAll(pattern)) {
          const wording = match[2];
          const staticWording = wording.replace(/\$\{[^}]*\}/g, "");
          if (!wording || wording.includes("t(") || !/[A-Za-zÀ-ỹ]{2}/u.test(staticWording)) continue;
          const line = source.slice(0, match.index).split("\n").length;
          violations.push(`${relativePath}:${line}: ${wording}`);
        }
      }
    }
    expect(violations).toEqual([]);
  });

  it("keeps the application shell wording behind localization keys", async () => {
    const source = await readFile(resolve(sourceRoot, "main.ts"), "utf8");
    expect(source).toContain('aria-label="${t("login.aria")}"');
    expect(source).toContain('${t("loading.title")}');
    expect(source).toContain('label: "menu.hunters"');
    expect(source).not.toContain('>Sign in<');
    expect(source).not.toContain('>Coming soon<');
  });
});
