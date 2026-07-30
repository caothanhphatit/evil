import { afterEach, describe, expect, it, vi } from "vitest";
import { recordClientEvent } from "./client-telemetry";

describe("client telemetry", () => {
  afterEach(() => vi.restoreAllMocks());

  it("emits structured events without serializing arbitrary errors", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const event = recordClientEvent("warn", "intent_rejected", { intent: "set_material_request", reason: "insufficient_gold" });

    expect(event).toMatchObject({ component: "web", level: "warn", event: "intent_rejected" });
    expect(event.fields).toEqual({ intent: "set_material_request", reason: "insufficient_gold" });
    expect(warn).toHaveBeenCalledWith(event);
  });
});
