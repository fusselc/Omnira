import { describe, expect, it } from "vitest";
import { truncateToBudget } from "./chat";

function msg(role: "user" | "assistant", content: string) {
  return { role, content };
}

describe("truncateToBudget", () => {
  it("keeps everything and reports no truncation when it all fits", () => {
    const history = [msg("user", "hi"), msg("assistant", "hello")];
    const { messages, truncated } = truncateToBudget(history, 1000);
    expect(truncated).toBe(false);
    expect(messages).toEqual([
      { role: "user", content: "hi" },
      { role: "assistant", content: "hello" },
    ]);
  });

  it("keeps the newest messages and drops the oldest when over budget", () => {
    const history = [
      msg("user", "first message, long ago"),
      msg("assistant", "first reply, long ago"),
      msg("user", "most recent question"),
      msg("assistant", "most recent answer"),
    ];
    // Budget only large enough for the last two messages plus overhead.
    const budget = "most recent question".length + "most recent answer".length + 40;
    const { messages, truncated } = truncateToBudget(history, budget);
    expect(truncated).toBe(true);
    expect(messages).toEqual([
      { role: "user", content: "most recent question" },
      { role: "assistant", content: "most recent answer" },
    ]);
  });

  it("never drops the single latest message even if it alone exceeds the budget", () => {
    const history = [
      msg("user", "short"),
      msg("assistant", "a".repeat(500)),
    ];
    const { messages, truncated } = truncateToBudget(history, 10);
    expect(messages).toEqual([{ role: "assistant", content: "a".repeat(500) }]);
    expect(truncated).toBe(true);
  });

  it("returns no messages and no truncation for empty history", () => {
    const { messages, truncated } = truncateToBudget([], 1000);
    expect(messages).toEqual([]);
    expect(truncated).toBe(false);
  });
});
