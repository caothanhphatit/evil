import type { ConnectionStatus } from "../net/world-client";
import { apiBaseUrlFor } from "../net/world-client";
import type { OriginalFlowSnapshot } from "../generated/protocol";
import { projectEntryPresentation, type EntryPhase } from "../ui/entry-flow";
import { t, type MessageKey } from "../i18n";
import { recordClientEvent } from "../observability/client-telemetry";

function element<T extends HTMLElement>(selector: string): T {
  const value = document.querySelector<T>(selector);
  if (!value) throw new Error(t("error.missing_element", { selector }));
  return value;
}

export class EntryController {
  private readonly loginScreen = element<HTMLElement>("#login-screen");
  private readonly villageScreen = element<HTMLElement>("#village-screen");
  private readonly rosterScreen = element<HTMLElement>("#roster-screen");
  private readonly bottomMenu = element<HTMLElement>("#bottom-menu");
  private readonly transition = element<HTMLElement>("#loading-transition");
  private readonly enterVillage = element<HTMLButtonElement>("#enter-village");
  private readonly loginEmail = element<HTMLInputElement>("#login-email");
  private readonly loginPassword = element<HTMLInputElement>("#login-password");
  private readonly gameLoadingScreen = element<HTMLElement>("#game-loading-screen");
  private readonly gameLoadingStatus = element<HTMLElement>("#game-loading-status");
  private readonly gameLoadingFill = element<HTMLElement>("#game-loading-fill");
  private readonly gameLoadingPercent = element<HTMLElement>("#game-loading-percent");
  private readonly gameLoadingRetry = element<HTMLButtonElement>("#game-loading-retry");
  private readonly bootStatus = element<HTMLElement>("#boot-status");
  private readonly loginReadiness = element<HTMLElement>(".login-readiness");
  private readonly mapLoadingFill = element<HTMLElement>("#map-loading-fill");
  private readonly mapLoadingLabel = element<HTMLElement>("#map-loading-label");
  private phase: EntryPhase = "login";
  private connectionState: ConnectionStatus = "connecting";
  private bootRequested = false;
  private mapReady = false;
  private mapLoadFailed = false;
  private hideTimer: number | undefined;
  private timeoutTimer: number | undefined;

  constructor(
    private readonly gameShell: HTMLElement,
    private readonly onStartRuntime: () => void,
    private readonly onCompleteBoot: () => void,
  ) {
    this.installAccountHandlers();
    this.gameLoadingRetry.addEventListener("click", () => location.reload());
    this.updateBootState();
  }

  private installAccountHandlers(): void {
    const toggle = element<HTMLButtonElement>("#register-account-toggle");
    const form = element<HTMLFormElement>("#register-account-form");
    const name = element<HTMLInputElement>("#register-account-name");
    const email = element<HTMLInputElement>("#register-account-email");
    const password = element<HTMLInputElement>("#register-account-password");
    const demo = element<HTMLSelectElement>("#demo-account-select");
    const cancel = element<HTMLButtonElement>("#register-account-cancel");
    demo.addEventListener("change", () => {
      if (!demo.value) return;
      this.loginEmail.value = demo.value;
      this.loginPassword.value = "Demo1234!";
    });
    toggle.addEventListener("click", () => {
      form.hidden = !form.hidden;
      if (!form.hidden) name.focus();
    });
    cancel.addEventListener("click", () => { form.hidden = true; });
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const displayName = name.value.trim();
      const normalizedEmail = email.value.trim().toLowerCase();
      if (displayName.length < 2 || !normalizedEmail.includes("@") || password.value.length < 8) {
        this.bootStatus.textContent = t("login.invalid_profile");
        return;
      }
      const authenticated = await this.authenticate("register", {
        display_name: displayName,
        email: normalizedEmail,
        password: password.value,
      });
      if (!authenticated) return;
      form.reset();
      form.hidden = true;
      this.beginLoading(displayName);
    });
    this.enterVillage.addEventListener("click", () => void this.signIn());
  }

  private async signIn(): Promise<void> {
    if (this.mapLoadFailed || this.phase !== "login") return;
    const email = this.loginEmail.value.trim().toLowerCase();
    if (!email || !this.loginPassword.value) {
      this.bootStatus.textContent = t("login.credentials_required");
      return;
    }
    const authenticated = await this.authenticate("login", {
      email,
      password: this.loginPassword.value,
    });
    if (!authenticated) return;
    this.beginLoading(email);
  }

  private async authenticate(action: "login" | "register", body: Record<string, string>): Promise<boolean> {
    this.enterVillage.disabled = true;
    try {
      const configured = window.__EVIL_HUNTER_CONFIG__?.apiBaseUrl || import.meta.env.VITE_WORLD_API_URL;
      const response = await fetch(`${apiBaseUrlFor(location, configured)}/account/${action}`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        this.bootStatus.textContent = response.status === 409
          ? t("login.account_exists")
          : response.status === 401
            ? t("login.invalid_credentials")
            : t("login.account_unavailable");
        return false;
      }
      return true;
    } catch {
      this.bootStatus.textContent = t("login.account_unavailable");
      return false;
    } finally {
      if (this.phase === "login") this.enterVillage.disabled = false;
    }
  }

  private beginLoading(account: string): void {
    this.phase = "loading";
    this.gameLoadingStatus.textContent = t("loading.account", {
      account,
    });
    this.setProgress(3);
    this.bootRequested = true;
    this.gameLoadingRetry.hidden = true;
    if (this.timeoutTimer !== undefined) clearTimeout(this.timeoutTimer);
    this.timeoutTimer = window.setTimeout(() => {
      if (this.phase === "loading") this.fail(t("loading.server_timeout"));
    }, 30_000);
    this.updateBootState();
    this.onStartRuntime();
    this.onCompleteBoot();
  }

  fail(message: string): void {
    recordClientEvent("error", "game_loading_failed", { phase: this.phase, message });
    this.mapLoadFailed = true;
    if (this.timeoutTimer !== undefined) clearTimeout(this.timeoutTimer);
    this.timeoutTimer = undefined;
    this.gameLoadingStatus.textContent = message;
    this.gameLoadingPercent.textContent = t("loading.error_title");
    this.gameLoadingFill.style.width = "100%";
    this.gameLoadingRetry.hidden = false;
    this.updateBootState();
  }

  updateMapProgress(loaded: number, total: number): void {
    const percent = total ? Math.round((loaded / total) * 100) : 0;
    const gamePercent = Math.min(88, Math.max(5, Math.round(5 + percent * .83)));
    this.mapLoadingFill.style.width = `${percent}%`;
    this.mapLoadingLabel.textContent = t("loading.map_progress", { percent });
    this.setProgress(gamePercent);
    this.gameLoadingStatus.textContent = t("loading.world_assets");
  }

  prepareHunters(): void {
    this.mapLoadingLabel.textContent = t("loading.preparing_game");
    this.mapLoadingFill.style.width = "100%";
    this.setProgress(92);
    this.gameLoadingStatus.textContent = t("loading.preparing_hunters");
  }

  markMapReady(snapshot: OriginalFlowSnapshot | null, latest: () => OriginalFlowSnapshot | null): void {
    this.loginReadiness.hidden = true;
    this.mapReady = true;
    if (snapshot) this.scheduleReveal(snapshot, latest);
    this.updateBootState();
  }

  scheduleReveal(snapshot: OriginalFlowSnapshot, latest: () => OriginalFlowSnapshot | null): void {
    if (!this.mapReady || this.phase !== "loading" || snapshot.screen === "boot") return;
    this.bootRequested = false;
    this.setProgress(100);
    this.gameLoadingStatus.textContent = t("loading.settlement_ready");
    if (this.timeoutTimer !== undefined) clearTimeout(this.timeoutTimer);
    this.timeoutTimer = undefined;
    if (this.hideTimer !== undefined) return;
    this.hideTimer = window.setTimeout(() => {
      this.phase = "game";
      this.hideTimer = undefined;
      this.updateBootState();
      const current = latest() ?? snapshot;
      this.syncScreens(current, this.rosterScreen.classList.contains("visible"));
    }, 850);
  }

  syncScreens(snapshot: OriginalFlowSnapshot, rosterOpen: boolean): boolean {
    const entry = projectEntryPresentation(this.phase);
    const village = entry.renderWorld && (snapshot.screen === "village" || snapshot.screen === "field");
    this.bottomMenu.hidden = !entry.enableGameUi || snapshot.screen === "boot";
    this.loginScreen.classList.toggle("leaving", !entry.showLogin);
    this.villageScreen.classList.toggle("visible", village || rosterOpen);
    this.villageScreen.classList.toggle("field-mode", snapshot.screen === "field");
    this.villageScreen.setAttribute("aria-hidden", String(!village && !rosterOpen));
    this.rosterScreen.classList.toggle("visible", rosterOpen);
    this.rosterScreen.setAttribute("aria-hidden", String(!rosterOpen));
    return village;
  }

  updateConnectionStatus(status: ConnectionStatus): void {
    this.connectionState = status;
    const labels: Record<ConnectionStatus, MessageKey> = {
      connecting: "connection.connecting", online: "connection.online", reconnecting: "connection.reconnecting", offline: "connection.offline",
    };
    const indicator = element<HTMLButtonElement>("#connection-status");
    indicator.className = `connection-status ${status}`;
    indicator.querySelector("span")!.textContent = t(labels[status]);
    this.updateBootState();
  }

  private setProgress(percent: number): void {
    this.gameLoadingFill.style.width = `${percent}%`;
    this.gameLoadingPercent.textContent = `${percent}%`;
  }

  private updateBootState(): void {
    this.enterVillage.disabled = this.phase !== "login" || this.mapLoadFailed;
    const entry = projectEntryPresentation(this.phase);
    this.gameShell.classList.toggle("entry-login", this.phase === "login");
    this.gameShell.classList.toggle("entry-loading", this.phase === "loading");
    this.gameShell.classList.toggle("entry-game", this.phase === "game");
    this.loginScreen.classList.toggle("leaving", !entry.showLogin);
    this.gameLoadingScreen.hidden = !entry.showLoading;
    if (!this.bootRequested) {
      this.transition.hidden = true;
      this.bootStatus.textContent = this.mapLoadFailed
        ? t("boot.map_unavailable")
        : !this.mapReady
          ? t("boot.preparing_assets")
          : this.connectionState === "online" ? t("boot.ready_sign_in") : t("boot.connecting_server");
      return;
    }
    const dispatching = this.connectionState === "online";
    this.transition.hidden = !dispatching;
    this.bootStatus.textContent = dispatching ? t("boot.entering_village") : t("boot.waiting_server");
  }
}

export async function preloadUiAssets(): Promise<void> {
  const images = [...document.images].map((image) => image.complete
    ? Promise.resolve()
    : new Promise<void>((resolve) => {
      image.addEventListener("load", () => resolve(), { once: true });
      image.addEventListener("error", () => resolve(), { once: true });
    }));
  await Promise.allSettled(images);
  await document.fonts.ready;
}
