export type EntryPhase = "login" | "loading" | "game";

export interface EntryPresentation {
  showLogin: boolean;
  showLoading: boolean;
  renderWorld: boolean;
  enableGameUi: boolean;
}

export function projectEntryPresentation(phase: EntryPhase): EntryPresentation {
  return {
    showLogin: phase === "login",
    showLoading: phase === "loading",
    renderWorld: phase !== "login",
    enableGameUi: phase === "game",
  };
}
