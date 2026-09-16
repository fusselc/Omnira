import { describe, expect, it } from "vitest";
import { acquire, release } from "./singleFlight";

describe("singleFlight", () => {
  it("rejects a second acquire until the first is released", () => {
    const lock = { current: false };
    expect(acquire(lock)).toBe(true);
    expect(acquire(lock)).toBe(false);
    expect(acquire(lock)).toBe(false);
    release(lock);
    expect(acquire(lock)).toBe(true);
  });

  it("blocks two claims in the same turn without waiting for a state update", () => {
    const lock = { current: false };
    const started: string[] = [];
    const tryStart = (label: string) => {
      if (!acquire(lock)) return;
      started.push(label);
    };
    tryStart("first");
    tryStart("second");
    expect(started).toEqual(["first"]);
  });
});
