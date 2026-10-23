import { describe, expect, it } from "vitest";
import { canSubmitDraft, composerDisabled, type ComposerLockInput } from "./chatUi";

const idle: ComposerLockInput = {
  canSend: true,
  generating: false,
  sending: false,
  startingModel: false,
};

describe("composerDisabled", () => {
  it("is enabled only when sending is allowed and nothing is in flight", () => {
    expect(composerDisabled(idle)).toBe(false);
  });

  it("locks while a send is starting, before generation is flagged", () => {
    expect(composerDisabled({ ...idle, sending: true })).toBe(true);
  });

  it("locks while a model switch is starting even if the old engine is still ready", () => {
    expect(composerDisabled({ ...idle, startingModel: true })).toBe(true);
  });

  it("locks while generating or when the send guard blocks", () => {
    expect(composerDisabled({ ...idle, generating: true })).toBe(true);
    expect(composerDisabled({ ...idle, canSend: false })).toBe(true);
  });
});

describe("canSubmitDraft", () => {
  it("requires non-whitespace text", () => {
    expect(canSubmitDraft(idle, "hello")).toBe(true);
    expect(canSubmitDraft(idle, "   \n ")).toBe(false);
  });

  it("refuses while the composer is locked", () => {
    expect(canSubmitDraft({ ...idle, sending: true }, "hello")).toBe(false);
  });
});
