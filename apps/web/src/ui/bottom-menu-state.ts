export function nextHunterRosterOpen(triggerIsBottomMenu: boolean, rosterActuallyOpen: boolean): boolean {
  return !triggerIsBottomMenu || !rosterActuallyOpen;
}
