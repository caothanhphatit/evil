export function nextHunterRosterOpen(triggerIsBottomMenu: boolean, rosterActuallyOpen: boolean): boolean {
  return !triggerIsBottomMenu || !rosterActuallyOpen;
}

export function syncWorldFocusMenu<T extends string>(screen: string, selected: T | null): T | "field" | null {
  if (screen === "field" && selected === null) return "field";
  if (screen === "village" && selected === "field") return null;
  return selected;
}
