import { describe, expect, it } from "vitest";
import { setPanelMessage } from "./panel-message";

describe("panel message", () => {
  it("keeps server-controlled feedback as text", () => {
    const title = { textContent: "" as string | null };
    const detail = { textContent: "" as string | null };
    const target = {
      querySelector: (selector: "b" | "span") => selector === "b" ? title : detail,
    };

    setPanelMessage(target, "Server response", '<img src=x onerror="alert(1)">');

    expect(title.textContent).toBe("Server response");
    expect(detail.textContent).toBe('<img src=x onerror="alert(1)">');
  });
});
