export type ClientTelemetryLevel = "info" | "warn" | "error";

export interface ClientTelemetryEvent {
  component: "web";
  event: string;
  level: ClientTelemetryLevel;
  timestamp: string;
  fields: Record<string, string | number | boolean | null>;
}

export function recordClientEvent(
  level: ClientTelemetryLevel,
  event: string,
  fields: ClientTelemetryEvent["fields"] = {},
): ClientTelemetryEvent {
  const payload: ClientTelemetryEvent = {
    component: "web",
    event,
    level,
    timestamp: new Date().toISOString(),
    fields,
  };
  console[level](payload);
  if (typeof window !== "undefined" && typeof CustomEvent !== "undefined") {
    window.dispatchEvent(new CustomEvent("evil:telemetry", { detail: payload }));
  }
  return payload;
}
