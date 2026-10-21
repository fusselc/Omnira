import { describe, expect, it, vi } from "vitest";
import { activateConversationModel } from "./modelActivation";
import type { RuntimeStatus } from "./ipc";

const status = (state: RuntimeStatus["state"], modelId: string | null): RuntimeStatus => ({
  state, model_id: modelId, engine_label: null, variant: null,
  accelerator_label: null, fallback_reason: null, port: null,
  context_size: null, last_error: null,
});

describe("conversation model activation", () => {
  it.each([
    status("stopped", null),
    status("starting", "model-a"),
    status("ready", "model-b"),
  ])("keeps the previous binding when activation returns $state/$model_id", async (result) => {
    const api = {
      startRuntime: vi.fn().mockResolvedValue(result),
      setConversationModel: vi.fn().mockResolvedValue(undefined),
    };
    expect(await activateConversationModel("model-a", "thread", api)).toBe(false);
    expect(api.startRuntime).toHaveBeenCalledWith("model-a");
    expect(api.setConversationModel).not.toHaveBeenCalled();
  });

  it("binds only after the requested model becomes ready", async () => {
    let finish!: (value: RuntimeStatus) => void;
    const api = {
      startRuntime: vi.fn(() => new Promise<RuntimeStatus>((resolve) => { finish = resolve; })),
      setConversationModel: vi.fn().mockResolvedValue(undefined),
    };
    const activation = activateConversationModel("model-a", "thread", api);
    expect(api.setConversationModel).not.toHaveBeenCalled();
    finish(status("ready", "model-a"));
    expect(await activation).toBe(true);
    expect(api.setConversationModel).toHaveBeenCalledExactlyOnceWith("thread", "model-a");
  });
});
