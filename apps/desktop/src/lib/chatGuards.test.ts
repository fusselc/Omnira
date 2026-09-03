import { describe, expect, it } from "vitest";
import type { ModelEntry, RuntimeStatus } from "./ipc";
import { canSendMessage, sendBlocker } from "./chatGuards";

const stopped: RuntimeStatus = {
  state: "stopped",
  engine_label: null,
  variant: null,
  accelerator_label: null,
  fallback_reason: null,
  model_id: null,
  port: null,
  context_size: null,
  last_error: null,
};

function ready(modelId: string): RuntimeStatus {
  return {
    ...stopped,
    state: "ready",
    engine_label: "llama.cpp",
    variant: "cpu",
    accelerator_label: "CPU",
    model_id: modelId,
    port: 12345,
    context_size: 8192,
  };
}

function model(id: string, status: ModelEntry["status"] = "ok"): ModelEntry {
  return {
    id,
    name: id,
    path: `C:\\models\\${id}.gguf`,
    file_size_bytes: 1,
    trained_context_length: 8192,
    last_used_at: null,
    added_at: "2026-01-01T00:00:00Z",
    status,
  };
}

describe("sendBlocker", () => {
  it("allows sending when the loaded model is registered and matches the thread", () => {
    expect(
      sendBlocker({
        runtime: ready("gemma"),
        models: [model("gemma")],
        conversationModelId: "gemma",
        generating: false,
      }),
    ).toBeNull();
  });

  it("allows a fresh thread with no model binding to use the loaded model", () => {
    expect(
      canSendMessage({
        runtime: ready("gemma"),
        models: [model("gemma")],
        conversationModelId: null,
        generating: false,
      }),
    ).toBe(true);
  });

  it("blocks while a response is already generating", () => {
    expect(
      sendBlocker({
        runtime: ready("gemma"),
        models: [model("gemma")],
        conversationModelId: "gemma",
        generating: true,
      }),
    ).toBe("generating");
  });

  it("blocks when no engine is running", () => {
    expect(
      sendBlocker({ runtime: stopped, models: [model("gemma")], conversationModelId: null, generating: false }),
    ).toBe("engine_not_ready");
    expect(
      sendBlocker({
        runtime: { ...stopped, state: "starting", engine_label: "llama.cpp" },
        models: [],
        conversationModelId: null,
        generating: false,
      }),
    ).toBe("engine_not_ready");
    expect(
      sendBlocker({ runtime: { ...stopped, state: "error" }, models: [], conversationModelId: null, generating: false }),
    ).toBe("engine_not_ready");
  });

  it("blocks a ready engine that somehow reports no model", () => {
    expect(
      sendBlocker({
        runtime: { ...ready("x"), model_id: null },
        models: [model("x")],
        conversationModelId: null,
        generating: false,
      }),
    ).toBe("no_model_loaded");
  });

  it("blocks when the loaded model was removed from Omnira (zombie engine)", () => {
    // The user deleted the GGUF entry while llama-server still had it in memory.
    expect(
      sendBlocker({
        runtime: ready("gemma"),
        models: [],
        conversationModelId: null,
        generating: false,
      }),
    ).toBe("loaded_model_unregistered");
  });

  it("blocks when the loaded model's file is missing or invalid", () => {
    expect(
      sendBlocker({
        runtime: ready("gemma"),
        models: [model("gemma", "missing")],
        conversationModelId: null,
        generating: false,
      }),
    ).toBe("loaded_model_unavailable");
    expect(
      sendBlocker({
        runtime: ready("gemma"),
        models: [model("gemma", "invalid")],
        conversationModelId: null,
        generating: false,
      }),
    ).toBe("loaded_model_unavailable");
  });

  it("blocks when the thread's model is unregistered, missing, or invalid", () => {
    const runtime = ready("gemma");
    expect(
      sendBlocker({ runtime, models: [model("gemma")], conversationModelId: "old", generating: false }),
    ).toBe("conversation_model_unregistered");
    expect(
      sendBlocker({
        runtime,
        models: [model("gemma"), model("old", "missing")],
        conversationModelId: "old",
        generating: false,
      }),
    ).toBe("conversation_model_missing");
    expect(
      sendBlocker({
        runtime,
        models: [model("gemma"), model("old", "invalid")],
        conversationModelId: "old",
        generating: false,
      }),
    ).toBe("conversation_model_invalid");
  });

  it("blocks when the thread's model is registered but a different one is loaded", () => {
    expect(
      sendBlocker({
        runtime: ready("gemma"),
        models: [model("gemma"), model("qwen")],
        conversationModelId: "qwen",
        generating: false,
      }),
    ).toBe("conversation_model_not_loaded");
  });
});
