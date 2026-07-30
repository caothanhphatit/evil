export type ShellUiMode = "wide" | "standard" | "compact" | "narrow";

export interface ShellViewportState {
  width: number;
  height: number;
  mode: ShellUiMode;
  short: boolean;
  veryShort: boolean;
  gearShort: boolean;
}

export function shouldBlockMobileLandscape(width: number, height: number, coarsePointer: boolean): boolean {
  return coarsePointer && width > height && Math.min(width, height) <= 600;
}

export function classifyShellViewport(width: number, height: number): ShellViewportState {
  const mode: ShellUiMode = width <= 390 ? "narrow" : width <= 520 ? "compact" : width <= 680 ? "standard" : "wide";
  return { width, height, mode, short: height <= 640, veryShort: height <= 560, gearShort: height <= 620 };
}

export function installShellViewportController(shell: HTMLElement): () => void {
  const coarsePointer = window.matchMedia("(pointer: coarse)");
  const sync = (): void => {
    const state = classifyShellViewport(shell.clientWidth, shell.clientHeight);
    shell.dataset.uiMode = state.mode;
    for (const mode of ["wide", "standard", "compact", "narrow"] as const) shell.classList.toggle(`ui-${mode}`, state.mode === mode);
    shell.classList.toggle("ui-short", state.short);
    shell.classList.toggle("ui-very-short", state.veryShort);
    shell.classList.toggle("ui-gear-short", state.gearShort);
    shell.classList.toggle(
      "mobile-landscape-blocked",
      shouldBlockMobileLandscape(state.width, state.height, coarsePointer.matches),
    );
  };
  const observer = new ResizeObserver(sync);
  observer.observe(shell);
  coarsePointer.addEventListener("change", sync);
  sync();
  return () => {
    observer.disconnect();
    coarsePointer.removeEventListener("change", sync);
  };
}
