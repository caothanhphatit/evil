import { MAX_MESSAGE_BYTES, PROTOCOL_VERSION } from "../generated/protocol";
import type { BottomMenuIntent, ClientCommand, ClientEnvelope, OriginalFlowSnapshot, ServerEnvelope, ServerMessage } from "../generated/protocol";

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
  private stopped = false;
  private connectionAttempt = 0;
  private readonly socketFactory: (url: string) => WebSocket;
  private readonly fetchFn: typeof fetch;
  private readonly uuidFactory: () => string;
  private readonly reconnectDelayMs: number;
  private readonly apiBaseUrl: string;
  private readonly webSocketUrl: string;
  private readonly envelopeSequencer = new EnvelopeSequencer();
  private readonly serverSequenceGuard = new ServerSequenceGuard();

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
  }

  connect(): void { this.stopped = false; void this.openSocket("connecting"); }
  disconnect(): void {
    this.stopped = true;
    this.connectionAttempt += 1;
    this.socket?.close();
    this.socket = null;
    if (this.reconnectTimer !== null) clearTimeout(this.reconnectTimer);
    this.reconnectTimer = null;
  }
  completeBoot(): boolean { return this.send({ type: "complete_boot" }); }
  selectBottomMenu(menu: BottomMenuIntent): boolean { return this.send({ type: "select_bottom_menu", menu }); }
  navigateBack(): boolean { return this.send({ type: "navigate_back" }); }
  enterField(): boolean { return this.send({ type: "enter_field" }); }
  selectEntity(entityId: string): boolean { return this.send({ type: "select_entity", entity_id: entityId }); }
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

      this.socket = this.socketFactory(this.webSocketUrl);
      this.socket.addEventListener("open", () => this.onStatus("online"));
      this.socket.addEventListener("message", (event) => this.handleMessage(event.data));
      this.socket.addEventListener("error", () => this.socket?.close());
      this.socket.addEventListener("close", () => this.scheduleReconnect());
    } catch (error) {
      console.warn("Session bootstrap or WebSocket connection failed.", error);
      if (attempt === this.connectionAttempt) this.scheduleReconnect();
    }
  }

  private handleMessage(data: unknown): void {
    try {
      const wire = String(data);
      if (new TextEncoder().encode(wire).byteLength > MAX_MESSAGE_BYTES) throw new Error("Server message exceeds protocol limit");
      const parsed = JSON.parse(wire) as unknown;
      if (!isServerEnvelope(parsed) || parsed.version !== PROTOCOL_VERSION || !this.serverSequenceGuard.accept(parsed.sequence)) throw new Error("Unsupported or out-of-order server envelope");
      const message = parsed.payload;
      if (message.type === "welcome") {
        this.envelopeSequencer.acceptWelcome(message.session_id);
        this.onPlayerIdentity(message.player_token);
        this.requestResync();
      }
      if (message.type === "intent_result") this.onIntentFeedback({ intent: message.intent, accepted: message.accepted, reason: message.reason });
      if (message.type === "binding_blocked") this.onBindingBlocked({ intent: message.intent, blockers: message.blockers });
      const snapshot = snapshotFromMessage(message);
      if (snapshot) this.onSnapshot(snapshot);
    } catch (error) { console.warn("Ignored malformed server message.", error); }
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
  const protocol = location.protocol === "https:" ? "https" : "http";
  return import.meta.env.VITE_WORLD_API_URL ?? `${protocol}://${location.hostname}:8080`;
}

function defaultWebSocketUrl(): string {
  const protocol = location.protocol === "https:" ? "wss" : "ws";
  return import.meta.env.VITE_WORLD_WS_URL ?? `${protocol}://${location.hostname}:8080/ws`;
}

function stripTrailingSlash(value: string): string { return value.replace(/\/+$/, ""); }

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
  if (message.type === "intent_result") return typeof message.intent === "string" && typeof message.accepted === "boolean" && isSnapshot(message.snapshot);
  return message.type === "binding_blocked" && typeof message.intent === "string" && Array.isArray(message.blockers) && isSnapshot(message.snapshot);
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
    && Array.isArray((snapshot.world as Record<string, unknown>).entities);
}
