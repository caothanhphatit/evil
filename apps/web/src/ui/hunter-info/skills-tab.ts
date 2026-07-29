import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";
import { t } from "../../i18n";

export function renderSkillsTab(info: HunterInfoView, useSkill?: (skillId: string) => void): HTMLElement {
  const root = node("section", "hunter-info-skills-tab");
  if (info.skills === null) return unavailable(t("hunter.skills.unavailable"));
  if (!info.skills.length) return unavailable(t("hunter.skills.empty"));
  const groups = new Map<string, typeof info.skills>();
  for (const skill of info.skills) {
    const group = skill.group ?? t("hunter.skills.group");
    groups.set(group, [...(groups.get(group) ?? []), skill]);
  }
  for (const [group, skills] of groups) {
    const section = node("section", "hunter-skill-group");
    section.append(node("h3", "", group));
    const grid = node("div");
    for (const skill of skills) {
      const card = node("article", `hunter-skill-card${skill.unlocked === false ? " locked" : ""}`);
      const icon = node("span", "hunter-skill-icon");
      if (skill.icon) icon.append(sourceImage(skill.icon));
      const copy = node("div");
      const title = node("header");
      title.append(node("b", "", skill.name));
      if (skill.level !== null) title.append(node("strong", "", t("common.level_short", { level: skill.level })));
      copy.append(title);
      if (skill.description) copy.append(node("p", "", skill.description));
      if (skill.unlocked === false && skill.unlockRequirement) copy.append(node("small", "", skill.unlockRequirement));
      if (skill.unlocked === true && useSkill) {
        const use = node("button", "hunter-skill-use", skill.ready === false
          ? t("hunter.skills.cooldown", { duration: formatCooldown(skill.cooldownRemainingMs) })
          : t("common.use"));
        use.type = "button";
        use.disabled = skill.ready === false;
        use.addEventListener("click", () => useSkill(skill.id));
        copy.append(use);
      }
      card.append(icon, copy);
      grid.append(card);
    }
    section.append(grid);
    root.append(section);
  }
  return root;
}

function formatCooldown(milliseconds: number | null): string {
  if (milliseconds === null) return t("hunter.skills.cooldown_unavailable");
  return `${Math.ceil(milliseconds / 100) / 10}s`;
}
