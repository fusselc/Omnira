/**
 * Pure "may we generate right now?" decision for the Chat composer.
 *
 * Generation must never start without a loaded model. The runtime status is
 * polled, so this is evaluated from the latest snapshot plus the model
 * registry: a runtime whose model was removed from Omnira (or whose file went
 * missing) is not a usable model even if llama-server is still up.
 */
import type { ModelEntry, RuntimeStatus } from "./ipc";

export type SendBlocker =
  | "generating"
  | "engine_not_ready"
  | "no_model_loaded"
  | "loaded_model_unregistered"
  | "loaded_model_unavailable"
  | "conversation_model_unregistered"
  | "conversation_model_missing"
  | "conversation_model_invalid"
  | "conversation_model_not_loaded";

export interface SendGuardInput {
  runtime: RuntimeStatus;
  models: ModelEntry[];
  /** Model persisted on the active conversation, if any. */
  conversationModelId: string | null;
  generating: boolean;
}

export function sendBlocker({
  runtime,
  models,
  conversationModelId,
  generating,
}: SendGuardInput): SendBlocker | null {
  if (generating) return "generating";
  if (runtime.state !== "ready") return "engine_not_ready";
  if (!runtime.model_id) return "no_model_loaded";

  const loaded = models.find((m) => m.id === runtime.model_id);
  if (!loaded) return "loaded_model_unregistered";
  if (loaded.status !== "ok") return "loaded_model_unavailable";

  if (conversationModelId != null) {
    const convoModel = models.find((m) => m.id === conversationModelId);
    if (!convoModel) return "conversation_model_unregistered";
    switch (convoModel.status) {
      case "missing":
        return "conversation_model_missing";
      case "invalid":
        return "conversation_model_invalid";
      case "ok":
        break;
      default: {
        const exhaustive: never = convoModel.status;
        return exhaustive;
      }
    }
    if (conversationModelId !== runtime.model_id) {
      return "conversation_model_not_loaded";
    }
  }

  return null;
}

export function canSendMessage(input: SendGuardInput): boolean {
  return sendBlocker(input) === null;
}
