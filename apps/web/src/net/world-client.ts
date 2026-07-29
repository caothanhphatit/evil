import { MAX_MESSAGE_BYTES, PROTOCOL_VERSION } from "../generated/protocol";
import type { BottomMenuIntent, ClientCommand, ClientEnvelope, FarmReport, OriginalFlowSnapshot, ServerEnvelope, ServerMessage } from "../generated/protocol";

export type ConnectionStatus = "connecting" | "online" | "reconnecting" | "offline";
export interface IntentFeedback { intent: string; accepted: boolean; reason: string | null }
export interface BindingBlockedFeedback { intent: string; blockers: string[] }

export interface WorldClientOptions {
  socketFactory?: (url: string) => WebSocket;
  fetchFn?: typeof fetch;
  uuidFactory?: () => string;
  reconnectDelayMs?: number;
  apiBaseUrl?: string;
  webSocketUrl?: string;
  onWorldFrame?: (snapshot: OriginalFlowSnapshot) => void;
}

export class EnvelopeSequencer {
  private sequence = 0;
  private sessionId: string | null = null;
  reset(): void { this.sequence = 0; this.sessionId = null; }
  acceptWelcome(sessionId: string): void { this.sessionId = sessionId; }
  isReady(): boolean { return this.sessionId !== null; }
  wrap(payload: ClientCommand, correlationId: string): ClientEnvelope {
    this.sequence += 1;
    return { version: PROTOCOL_VERSION, sequence: this.sequence, session_id: this.sessionId, correlation_id: correlationId, payload };
  }
}

export class ServerSequenceGuard {
  private last = 0;
  reset(): void { this.last = 0; }
  accept(sequence: number): boolean {
    if (!Number.isSafeInteger(sequence) || sequence !== this.last + 1) return false;
    this.last = sequence;
    return true;
  }
}

export class WorldClient {
  private socket: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private welcomeTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;
  private connectionAttempt = 0;
  private pendingBoot = false;
  private readonly socketFactory: (url: string) => WebSocket;
  private readonly fetchFn: typeof fetch;
  private readonly uuidFactory: () => string;
  private readonly reconnectDelayMs: number;
  private readonly apiBaseUrl: string;
  private readonly webSocketUrl: string;
  private readonly onWorldFrame: (snapshot: OriginalFlowSnapshot) => void;
  private readonly envelopeSequencer = new EnvelopeSequencer();
  private readonly serverSequenceGuard = new ServerSequenceGuard();
  private latestSnapshot: OriginalFlowSnapshot | null = null;

  constructor(
    private readonly onSnapshot: (snapshot: OriginalFlowSnapshot) => void,
    private readonly onStatus: (status: ConnectionStatus) => void,
    private readonly onIntentFeedback: (feedback: IntentFeedback) => void = () => undefined,
    private readonly onBindingBlocked: (feedback: BindingBlockedFeedback) => void = () => undefined,
    private readonly onPlayerIdentity: (playerToken: string) => void = () => undefined,
    options: WorldClientOptions = {},
  ) {
    this.socketFactory = options.socketFactory ?? ((url) => new WebSocket(url));
    this.fetchFn = options.fetchFn ?? ((input, init) => fetch(input, init));
    this.uuidFactory = options.uuidFactory ?? (() => crypto.randomUUID());
    this.reconnectDelayMs = options.reconnectDelayMs ?? 1500;
    this.apiBaseUrl = stripTrailingSlash(options.apiBaseUrl ?? defaultApiBaseUrl());
    this.webSocketUrl = options.webSocketUrl ?? defaultWebSocketUrl();
    this.onWorldFrame = options.onWorldFrame ?? this.onSnapshot;
  }

  connect(): void { this.stopped = false; void this.openSocket("connecting"); }
  submitFarmReport(report: FarmReport): boolean { return this.send({ type: "submit_farm_report", report }); }
  disconnect(): void {
    this.stopped = true;
    this.pendingBoot = false;
    this.connectionAttempt += 1;
    this.socket?.close();
    this.socket = null;
    if (this.reconnectTimer !== null) clearTimeout(this.reconnectTimer);
    this.reconnectTimer = null;
    if (this.welcomeTimer !== null) clearTimeout(this.welcomeTimer);
    this.welcomeTimer = null;
  }
  completeBoot(): boolean {
    if (this.stopped) return false;
    this.pendingBoot = true;
    this.send({ type: "complete_boot" });
    return true;
  }
  selectBottomMenu(menu: BottomMenuIntent): boolean { return this.send({ type: "select_bottom_menu", menu }); }
  navigateBack(): boolean { return this.send({ type: "navigate_back" }); }
  enterField(): boolean { return this.send({ type: "enter_field" }); }
  enterMonsterMap(mapId: string): boolean { return this.send({ type: "enter_monster_map", map_id: mapId }); }
  setMonsterDensity(level: number): boolean { return this.send({ type: "set_monster_density", level }); }
  setMonsterRegionDensity(regionId: string, level: number): boolean { return this.send({ type: "set_monster_region_density", region_id: regionId, level }); }
  selectMonsterTarget(monsterId: string, hunterId: number): boolean { return this.send({ type: "select_monster_target", monster_id: monsterId, hunter_id: hunterId }); }
  selectEntity(entityId: string): boolean { return this.send({ type: "select_entity", entity_id: entityId }); }
  constructBuilding(buildingId: string): boolean { return this.send({ type: "construct_building", building_id: buildingId }); }
  constructBuildingAt(buildingId: string, gridX: number, gridY: number): boolean { return this.send({ type: "construct_building_at", building_id: buildingId, grid_x: gridX, grid_y: gridY }); }
  upgradeBuilding(instanceId: string): boolean { return this.send({ type: "upgrade_building", instance_id: instanceId }); }
  moveBuilding(instanceId: string, gridX: number, gridY: number): boolean { return this.send({ type: "move_building", instance_id: instanceId, grid_x: gridX, grid_y: gridY }); }
  useBuilding(instanceId: string): boolean { return this.send({ type: "use_building", instance_id: instanceId }); }
  startBuildingService(instanceId: string, hunterId: number, productId: string): boolean { return this.send({ type: "start_building_service", instance_id: instanceId, hunter_id: hunterId, product_id: productId }); }
  setMaterialRequest(instanceId: string, materialId: string, quantity: number): boolean { return this.send({ type: "set_material_request", instance_id: instanceId, material_id: materialId, quantity }); }
  cancelMaterialRequest(instanceId: string, materialId: string): boolean { return this.send({ type: "cancel_material_request", instance_id: instanceId, material_id: materialId }); }
  craftShopItem(instanceId: string, recipeId: string, quantity: number, materialId: string | null = null): boolean { return this.send({ type: "craft_shop_item", instance_id: instanceId, recipe_id: recipeId, material_id: materialId, quantity }); }
  purchaseShopItem(hunterId: number, shopId: string, productId: string): boolean { return this.send({ type: "purchase_shop_item", hunter_id: hunterId, shop_id: shopId, product_id: productId }); }
  sellShopItem(shopId: string, productId: string): boolean { return this.send({ type: "sell_shop_item", shop_id: shopId, product_id: productId }); }
  openHunterProgression(hunterId: number): boolean { return this.send({ type: "open_hunter_progression", hunter_id: hunterId }); }
  assignHunterHunt(hunterId: number, zoneId: string): boolean { return this.send({ type: "assign_hunter_hunt", hunter_id: hunterId, zone_id: zoneId }); }
  returnHunterHunt(hunterId: number): boolean { return this.send({ type: "return_hunter_hunt", hunter_id: hunterId }); }
  sellHunterLoot(hunterId: number): boolean { return this.send({ type: "sell_hunter_loot", hunter_id: hunterId }); }
  startHunterEnhancement(hunterId: number): boolean { return this.send({ type: "start_hunter_enhancement", hunter_id: hunterId }); }
  reviveHunter(hunterId: number): boolean { return this.send({ type: "revive_hunter", hunter_id: hunterId }); }
  learnHunterSkill(hunterId: number, skillId: string): boolean { return this.send({ type: "learn_hunter_skill", hunter_id: hunterId, skill_id: skillId }); }
  useHunterSkill(hunterId: number, skillId: string, targetEntityId: string | null = null): boolean {
    return this.send({ type: "use_hunter_skill", hunter_id: hunterId, skill_id: skillId, target_entity_id: targetEntityId });
  }
  banishHunter(hunterId: number): boolean { return this.send({ type: "banish_hunter", hunter_id: hunterId }); }
  equipHunterItem(hunterId: number, itemId: number): boolean { return this.send({ type: "equip_hunter_item", hunter_id: hunterId, item_id: itemId }); }
  enhanceHunterGear(
    hunterId: number,
    gearInstanceId: string,
    mode: "single" | "to_10" | "to_15" | "to_20",
    optionalMaterialIds: string[] = [],
  ): boolean {
    return this.send({ type: "enhance_hunter_gear", hunter_id: hunterId, gear_instance_id: gearInstanceId, mode, optional_material_ids: optionalMaterialIds });
  }
  requestResync(): boolean { return this.send({ type: "request_resync" }); }

  private async openSocket(status: ConnectionStatus): Promise<void> {
    const attempt = ++this.connectionAttempt;
    this.onStatus(status);
    this.envelopeSequencer.reset();
    this.serverSequenceGuard.reset();
    try {
      const response = await this.fetchFn(`${this.apiBaseUrl}/session/bootstrap`, {
        method: "POST",
        credentials: "include",
        headers: { Accept: "application/json" },
      });
      if (!response.ok) throw new Error(`Session bootstrap failed with HTTP ${response.status}`);
      if (this.stopped || attempt !== this.connectionAttempt) return;

      const socket = this.socketFactory(this.webSocketUrl);
      this.socket = socket;
      socket.addEventListener("open", () => {
        if (this.socket !== socket) return;
        if (this.welcomeTimer !== null) clearTimeout(this.welcomeTimer);
        this.welcomeTimer = setTimeout(() => {
          if (this.socket !== socket || this.envelopeSequencer.isReady()) return;
          console.warn("WebSocket opened but the server welcome timed out.");
          this.failProtocol(socket);
        }, 10000);
      });
      socket.addEventListener("message", (event) => {
        if (this.socket === socket) this.handleMessage(event.data, socket);
      });
      socket.addEventListener("error", () => {
        if (this.socket === socket) socket.close();
      });
      socket.addEventListener("close", () => {
        if (this.socket !== socket) return;
        this.socket = null;
        if (this.welcomeTimer !== null) clearTimeout(this.welcomeTimer);
        this.welcomeTimer = null;
        this.scheduleReconnect();
      });
    } catch (error) {
      console.warn("Session bootstrap or WebSocket connection failed.", error);
      if (attempt === this.connectionAttempt) this.scheduleReconnect();
    }
  }

  private handleMessage(data: unknown, socket: WebSocket): void {
    try {
      const wire = String(data);
      if (new TextEncoder().encode(wire).byteLength > MAX_MESSAGE_BYTES) throw new Error("Server message exceeds protocol limit");
      const parsed = JSON.parse(wire) as unknown;
      if (!isServerEnvelope(parsed) || parsed.version !== PROTOCOL_VERSION || !this.serverSequenceGuard.accept(parsed.sequence)) throw new Error("Unsupported or out-of-order server envelope");
      const message = parsed.payload;
      if (message.type === "welcome") {
        if (this.welcomeTimer !== null) clearTimeout(this.welcomeTimer);
        this.welcomeTimer = null;
        this.envelopeSequencer.acceptWelcome(message.session_id);
        this.onPlayerIdentity(message.player_token);
        this.onStatus("online");
        this.requestResync();
        if (message.snapshot.screen === "boot") this.flushPendingBoot();
        else this.pendingBoot = false;
      }
      if (message.type === "intent_result") this.onIntentFeedback({ intent: message.intent, accepted: message.accepted, reason: message.reason });
      if (message.type === "binding_blocked") this.onBindingBlocked({ intent: message.intent, blockers: message.blockers });
      if (message.type === "world_frame") {
        if (this.latestSnapshot) {
          this.latestSnapshot = { ...this.latestSnapshot, world: message.world };
          this.onWorldFrame(this.latestSnapshot);
        }
        return;
      }
      const snapshot = snapshotFromMessage(message);
      if (snapshot) {
        this.latestSnapshot = snapshot;
        if (snapshot.screen !== "boot") this.pendingBoot = false;
        this.onSnapshot(snapshot);
      }
    } catch (error) {
      console.warn("Protocol fault; reconnecting for a clean resync.", error);
      this.failProtocol(socket);
    }
  }

  private flushPendingBoot(): void {
    if (this.pendingBoot && this.send({ type: "complete_boot" })) this.pendingBoot = false;
  }

  private failProtocol(socket: WebSocket): void {
    if (this.socket !== socket) return;
    this.socket = null;
    if (this.welcomeTimer !== null) clearTimeout(this.welcomeTimer);
    this.welcomeTimer = null;
    this.envelopeSequencer.reset();
    this.serverSequenceGuard.reset();
    socket.close(4002, "Protocol error");
    this.scheduleReconnect();
  }

  private send(command: ClientCommand): boolean {
    if (this.socket?.readyState !== WebSocket.OPEN || !this.envelopeSequencer.isReady()) return false;
    this.socket.send(JSON.stringify(this.envelopeSequencer.wrap(command, this.uuidFactory())));
    return true;
  }

  private scheduleReconnect(): void {
    if (this.stopped || this.reconnectTimer !== null) { if (this.stopped) this.onStatus("offline"); return; }
    this.onStatus("reconnecting");
    this.reconnectTimer = setTimeout(() => { this.reconnectTimer = null; void this.openSocket("reconnecting"); }, this.reconnectDelayMs);
  }
}

function defaultApiBaseUrl(): string {
  return apiBaseUrlFor(location, window.__EVIL_HUNTER_CONFIG__?.apiBaseUrl || import.meta.env.VITE_WORLD_API_URL);
}

function defaultWebSocketUrl(): string {
  return webSocketUrlFor(location, window.__EVIL_HUNTER_CONFIG__?.webSocketUrl || import.meta.env.VITE_WORLD_WS_URL);
}

export function apiBaseUrlFor(currentLocation: Pick<Location, "origin">, configured?: string): string {
  return stripTrailingSlash(configured?.trim() || currentLocation.origin);
}

export function webSocketUrlFor(currentLocation: Pick<Location, "protocol" | "host">, configured?: string): string {
  if (configured?.trim()) return configured.trim();
  const protocol = currentLocation.protocol === "https:" ? "wss" : "ws";
  return `${protocol}://${currentLocation.host}/ws`;
}

function stripTrailingSlash(value: string): string { return value.replace(/\/+$/, ""); }

declare global {
  interface Window {
    __EVIL_HUNTER_CONFIG__?: { apiBaseUrl?: string; webSocketUrl?: string };
  }
}

function snapshotFromMessage(message: ServerMessage): OriginalFlowSnapshot | null {
  if (message.type === "welcome" || message.type === "resync" || message.type === "world_update" || message.type === "intent_result" || message.type === "binding_blocked") return message.snapshot;
  return null;
}

function isServerMessage(value: unknown): value is ServerMessage {
  if (typeof value !== "object" || value === null || !("type" in value)) return false;
  const message = value as Record<string, unknown>;
  if (message.type === "welcome") return typeof message.player_token === "string" && typeof message.session_id === "string" && isSnapshot(message.snapshot);
  if (message.type === "resync") return isSnapshot(message.snapshot);
  if (message.type === "world_update") return isSnapshot(message.snapshot);
  if (message.type === "world_frame") return isWorldProjection(message.world);
  if (message.type === "farm_report_queued") return typeof message.window_id === "number";
  if (message.type === "intent_result") return typeof message.intent === "string" && typeof message.accepted === "boolean" && isSnapshot(message.snapshot);
  return message.type === "binding_blocked" && typeof message.intent === "string" && Array.isArray(message.blockers) && isSnapshot(message.snapshot);
}

function isWorldProjection(value: unknown): boolean {
  if (typeof value !== "object" || value === null) return false;
  const world = value as Record<string, unknown>;
  return typeof world.visual_tick === "number"
    && Array.isArray(world.entities)
    && Array.isArray(world.drops)
    && Array.isArray(world.combat_presentations);
}

function isServerEnvelope(value: unknown): value is ServerEnvelope {
  if (typeof value !== "object" || value === null) return false;
  const envelope = value as Record<string, unknown>;
  return typeof envelope.version === "number" && typeof envelope.sequence === "number" && typeof envelope.session_id === "string"
    && (envelope.correlation_id === null || typeof envelope.correlation_id === "string") && isServerMessage(envelope.payload);
}

function isSnapshot(value: unknown): value is OriginalFlowSnapshot {
  if (typeof value !== "object" || value === null) return false;
  const snapshot = value as Record<string, unknown>;
  return ["boot", "village", "hunter_roster", "field"].includes(snapshot.screen as string)
    && snapshot.content_release_id === "original-flow-v1" && Array.isArray(snapshot.flow_order)
    && typeof snapshot.village === "object" && snapshot.village !== null
    && typeof snapshot.hunter_roster === "object" && snapshot.hunter_roster !== null
    && typeof snapshot.field === "object" && snapshot.field !== null
    && typeof snapshot.world === "object" && snapshot.world !== null
    && Array.isArray((snapshot.world as Record<string, unknown>).entities)
    && Array.isArray((snapshot.world as Record<string, unknown>).drops)
    && Array.isArray((snapshot.world as Record<string, unknown>).combat_presentations);
}
