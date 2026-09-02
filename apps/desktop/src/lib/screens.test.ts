import { describe, expect, it } from "vitest";
import { isScreenMounted, PERSISTENT_SCREENS, type Screen } from "./screens";

const ALL: Screen[] = ["chat", "models", "settings", "diagnostics"];

describe("isScreenMounted", () => {
  it("keeps Chat mounted whichever screen is active", () => {
    for (const active of ALL) {
      expect(isScreenMounted("chat", active)).toBe(true);
    }
  });

  it("mounts the other screens only while they are active", () => {
    for (const screen of ALL.filter((s) => !PERSISTENT_SCREENS.includes(s))) {
      for (const active of ALL) {
        expect(isScreenMounted(screen, active)).toBe(screen === active);
      }
    }
  });

  it("only Chat is persistent (the stream owner)", () => {
    expect(PERSISTENT_SCREENS).toEqual(["chat"]);
  });
});
