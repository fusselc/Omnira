import { describe, expect, it } from "vitest";
import {
  canSubmitDraft,
  composerDisabled,
  composerPlaceholder,
  isNearBottom,
  isSubmitKey,
  replyAnnouncement,
  retryModelId,
  type ComposerLockInput,
  type ComposerPlaceholderInput,
} from "./chatUi";

const idle: ComposerLockInput = {
  canSend: true,
  generating: false,
  sending: false,
  startingModel: false,
  loadingThread: false,
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

  it("locks until the selected conversation's history has loaded", () => {
    expect(composerDisabled({ ...idle, loadingThread: true })).toBe(true);
  });

  it("locks while generating or when the send guard blocks", () => {
    expect(composerDisabled({ ...idle, generating: true })).toBe(true);
    expect(composerDisabled({ ...idle, canSend: false })).toBe(true);
  });
});

describe("isNearBottom", () => {
  const view = (scrollTop: number) => ({ scrollHeight: 1000, clientHeight: 400, scrollTop });

  it("follows when at or just above the end", () => {
    expect(isNearBottom(view(600))).toBe(true);
    expect(isNearBottom(view(540))).toBe(true);
  });

  it("stops following once the user scrolls up past the threshold", () => {
    expect(isNearBottom(view(536))).toBe(false);
    expect(isNearBottom(view(0))).toBe(false);
  });

  it("treats content shorter than the viewport as at the end", () => {
    expect(isNearBottom({ scrollHeight: 300, clientHeight: 400, scrollTop: 0 })).toBe(true);
  });
});

describe("isSubmitKey", () => {
  const key = (overrides: Partial<Parameters<typeof isSubmitKey>[0]>) => ({
    key: "Enter",
    shiftKey: false,
    isComposing: false,
    keyCode: 13,
    ...overrides,
  });

  it("sends on plain Enter", () => {
    expect(isSubmitKey(key({}))).toBe(true);
  });

  it("keeps Shift+Enter as a newline", () => {
    expect(isSubmitKey(key({ shiftKey: true }))).toBe(false);
  });

  it("ignores Enter that confirms an IME composition", () => {
    expect(isSubmitKey(key({ isComposing: true }))).toBe(false);
    expect(isSubmitKey(key({ keyCode: 229 }))).toBe(false);
  });

  it("ignores other keys", () => {
    expect(isSubmitKey(key({ key: "a", keyCode: 65 }))).toBe(false);
  });
});

describe("replyAnnouncement", () => {
  it("announces thinking, then generating, without streaming tokens", () => {
    expect(replyAnnouncement(true, false, null)).toBe("Thinking...");
    expect(replyAnnouncement(true, true, "complete")).toBe("Generating a response");
  });

  it("announces how the last reply ended once generation finishes", () => {
    expect(replyAnnouncement(false, false, "complete")).toBe("Response complete.");
    expect(replyAnnouncement(false, false, "length")).toMatch(/length limit/);
    expect(replyAnnouncement(false, false, "stopped")).toMatch(/Partial response kept/);
    expect(replyAnnouncement(false, false, "failed")).toMatch(/could not be completed/);
  });

  it("stays silent before any reply", () => {
    expect(replyAnnouncement(false, false, null)).toBe("");
  });
});

describe("composerPlaceholder", () => {
  const ready: ComposerPlaceholderInput = {
    runtimeState: "ready",
    generating: false,
    viewingStream: false,
    startingModel: false,
    conversationModelUnregistered: false,
    conversationModelFileMissing: false,
    conversationModelInvalid: false,
    needsRuntimeReload: false,
    loadedModelUnavailable: false,
    engineErrorMessage: null,
  };

  it("invites a message when ready", () => {
    expect(composerPlaceholder(ready)).toBe("Message your local model...");
  });

  it("explains an invalid conversation model instead of inviting a message", () => {
    expect(composerPlaceholder({ ...ready, conversationModelInvalid: true })).toMatch(
      /model file is invalid/,
    );
  });

  it("says the model is starting rather than asking to load it again", () => {
    expect(
      composerPlaceholder({ ...ready, runtimeState: "starting", needsRuntimeReload: true }),
    ).toBe("Starting model...");
    expect(composerPlaceholder({ ...ready, startingModel: true })).toBe("Starting model...");
  });

  it("explains a reply in progress here or in another conversation", () => {
    expect(composerPlaceholder({ ...ready, generating: true, viewingStream: true })).toMatch(
      /press Stop/,
    );
    expect(composerPlaceholder({ ...ready, generating: true })).toMatch(
      /Another conversation/,
    );
  });

  it("uses the engine's normalized message on error", () => {
    expect(
      composerPlaceholder({
        ...ready,
        runtimeState: "error",
        engineErrorMessage: "The chat engine could not start.",
      }),
    ).toBe("The chat engine could not start.");
    expect(composerPlaceholder({ ...ready, runtimeState: "error" })).toBe(
      "Omnira's engine is not responding.",
    );
  });

  it("asks for a model when nothing is running", () => {
    expect(composerPlaceholder({ ...ready, runtimeState: "stopped" })).toBe(
      "Choose a model to start",
    );
  });

  it("never mentions accelerators", () => {
    const states: ComposerPlaceholderInput["runtimeState"][] = [
      "ready",
      "starting",
      "error",
      "stopped",
    ];
    for (const runtimeState of states) {
      expect(composerPlaceholder({ ...ready, runtimeState })).not.toMatch(/GPU|CUDA|Vulkan/i);
    }
  });
});

describe("retryModelId", () => {
  const models = [
    { id: "a", status: "ok" as const },
    { id: "b", status: "missing" as const },
    { id: "c", status: "ok" as const },
  ];

  it("prefers the conversation's model when it is usable", () => {
    expect(retryModelId(models, "a", "c")).toBe("a");
  });

  it("falls back to the last engine model when the conversation has none usable", () => {
    expect(retryModelId(models, null, "c")).toBe("c");
    expect(retryModelId(models, "b", "c")).toBe("c");
  });

  it("returns null when nothing usable is known", () => {
    expect(retryModelId(models, "b", null)).toBeNull();
    expect(retryModelId(models, "gone", "also-gone")).toBeNull();
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
