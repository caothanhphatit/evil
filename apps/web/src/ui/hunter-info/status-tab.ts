import { hunterPercent } from "../hunter-roster";
import { node, sourceImage } from "./dom";
import type { HunterInfoView } from "./model";

const HUD = "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-status";
const STATUS_ICONS = ["h_detail_ic_01__1826.png", "h_detail_ic_02__3675.png", "h_detail_ic_03__6850.png", "h_detail_ic_04__4869.png"];
const STAT_ICONS = ["h_detail_ic_05__2429.png", "h_detail_ic_06__6625.png", "h_detail_ic_07__2675.png", "h_detail_ic_08__2267.png", "h_detail_ic_09__2988.png"];

export function renderStatusTab(info: HunterInfoView): HTMLElement {
  const root = node("section", "hunter-info-status-tab");
  const primary = node("div", "hunter-status-primary");
  const summary = node("header", "hunter-status-summary");
  summary.append(node("b", "hunter-rarity", info.hunter.rarityName ?? "Rarity unavailable"));
  const levelClass = [info.hunter.level === null ? null : `Lv.${info.hunter.level}`, info.hunter.className ?? info.hunter.classFamily].filter(Boolean).join(" ");
  if (levelClass) summary.append(node("strong", "", levelClass));
  primary.append(summary);

  const needs = node("div", "hunter-status-needs");
  const rows: Array<[string, number | null, number | null, string, string]> = [
    ["HP", info.hunter.hp, info.hunter.maxHp, "hp", STATUS_ICONS[0]],
    ["Satiety", info.hunter.satiety, info.hunter.maxSatiety, "satiety", STATUS_ICONS[1]],
    ["Mood", info.hunter.mood, info.hunter.maxMood, "mood", STATUS_ICONS[2]],
    ["Stamina", info.hunter.stamina, info.hunter.maxStamina, "stamina", STATUS_ICONS[3]],
  ];
  for (const [label, current, maximum, kind, icon] of rows) needs.append(gauge(label, current, maximum, kind, `${HUD}/${icon}`));
  primary.append(needs);
  const awakening = info.awakening
    ? `Awakening ${integer(info.awakening.current)}/${integer(info.awakening.maximum)}`
    : "Awakening unavailable";
  primary.append(node("div", `hunter-awakening${info.awakening ? "" : " unresolved"}`, awakening));
  root.append(primary);

  const combat = node("div", "hunter-status-combat");
  const dps = node("header", `hunter-dps${info.dps === null ? " unresolved" : ""}`);
  dps.append(node("span", "", "DPS"), node("b", "", info.dps === null ? "—" : format(info.dps)));
  combat.append(dps);
  const values = node("div", "hunter-combat-values");
  const stats: Array<[string, number | null, string, (value: number) => string]> = [
    ["ATK", info.hunter.attack, STAT_ICONS[0], integer],
    ["DEF", info.hunter.defense, STAT_ICONS[1], integer],
    ["CRIT", info.criticalChance, STAT_ICONS[2], percent],
    ["ATK SPD", info.attackSpeed, STAT_ICONS[3], twoDecimals],
    ["Evasion", info.evasion, STAT_ICONS[4], percent],
  ];
  for (const [label, value, icon, formatter] of stats) {
    const row = node("div", `hunter-combat-row${value === null ? " unresolved" : ""}`);
    row.append(sourceImage(`${HUD}/${icon}`), node("span", "", label), node("b", value === null ? "" : label === "ATK" ? "attack" : label === "DEF" ? "defense" : "", value === null ? "—" : formatter(value)));
    values.append(row);
  }
  combat.append(values);
  root.append(combat);
  return root;
}

function gauge(label: string, current: number | null, maximum: number | null, kind: string, icon: string): HTMLElement {
  const row = node("div", `hunter-info-gauge ${kind}`);
  row.append(sourceImage(icon));
  const content = node("div");
  const heading = node("span");
  heading.append(node("b", "", label), node("strong", "", current === null || maximum === null ? "Unavailable" : `${integer(current)}/${integer(maximum)}`));
  const track = node("i");
  const fill = node("i");
  fill.style.width = `${hunterPercent(current, maximum) ?? 0}%`;
  track.append(fill);
  content.append(heading, track);
  row.append(content);
  return row;
}

function format(value: number): string { return Number.isInteger(value) ? String(value) : value.toFixed(2); }
function integer(value: number): string { return Math.round(value).toString(); }
function percent(value: number): string { return `${format(value)}%`; }
function twoDecimals(value: number): string { return value.toFixed(2); }
