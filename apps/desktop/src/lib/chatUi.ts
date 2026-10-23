/**
 * Pure presentation decisions for the Chat screen, kept out of the component
 * so they can be unit tested without mounting React.
 */
import type { ModelEntry, RuntimeState } from "./ipc";

export interface ComposerLockInput {
  /** `sendBlocker(...) === null` for the latest runtime snapshot. */
  canSend: boolean;
  generating: boolean;
  /** A send is between the click and the stream starting. */
  sending: boolean;
  /** The user picked a model and its engine is being started. */
  startingModel: boolean;
  /**
   * The selected conversation's messages have not arrived yet. Sending now
   * would build context from the wrong (or empty) history.
   */
  loadingThread: boolean;
}

/** Whether the composer textarea must refuse input right now. */
export function composerDisabled({
  canSend,
  generating,
  sending,
  startingModel,
  loadingThread,
}: ComposerLockInput): boolean {
  return !canSend || generating || sending || startingModel || loadingThread;
}

/** Distance from the bottom within which the thread keeps following new text. */
export const FOLLOW_THRESHOLD_PX = 64;

/** Whether the thread is scrolled close enough to the end to keep following. */
export function isNearBottom(
  el: Pick<HTMLElement, "scrollHeight" | "scrollTop" | "clientHeight">,
  threshold = FOLLOW_THRESHOLD_PX,
): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight < threshold;
}

/**
 * Enter sends, Shift+Enter inserts a newline, and Enter that confirms an IME
 * composition (keyCode 229 in WebView2 / Chromium) must never send.
 */
export function isSubmitKey(
  e: Pick<KeyboardEvent, "key" | "shiftKey" | "isComposing" | "keyCode">,
): boolean {
  return e.key === "Enter" && !e.shiftKey && !e.isComposing && e.keyCode !== 229;
}

/** How the most recent reply ended, for the screen-reader status line. */
export type ReplyOutcome = "complete" | "length" | "stopped" | "failed";

/**
 * Text for the polite live region. Announces phase changes only (never the
 * streamed tokens) so a screen reader is not flooded mid-reply.
 */
export function replyAnnouncement(
  generating: boolean,
  hasStreamedText: boolean,
  lastOutcome: ReplyOutcome | null,
): string {
  if (generating) return hasStreamedText ? "Generating a response" : "Thinking...";
  if (lastOutcome === null) return "";
  switch (lastOutcome) {
    case "complete":
      return "Response complete.";
    case "length":
      return "Response complete. It reached the length limit.";
    case "stopped":
      return "Generation stopped. Partial response kept.";
    case "failed":
      return "The response could not be completed.";
    default: {
      const exhaustive: never = lastOutcome;
      return exhaustive;
    }
  }
}

export interface ComposerPlaceholderInput {
  runtimeState: RuntimeState;
  /** Any conversation is generating. */
  generating: boolean;
  /** The generating conversation is the one on screen. */
  viewingStream: boolean;
  /** A model switch requested from Chat is in flight. */
  startingModel: boolean;
  conversationModelUnregistered: boolean;
  conversationModelFileMissing: boolean;
  conversationModelInvalid: boolean;
  needsRuntimeReload: boolean;
  loadedModelUnavailable: boolean;
  engineErrorMessage: string | null;
}

/**
 * Composer placeholder that says why the box is disabled, or what to do next.
 * Order matters: in-flight work first, then the conversation's own model, then
 * the engine.
 */
export function composerPlaceholder(s: ComposerPlaceholderInput): string {
  if (s.generating) {
    return s.viewingStream
      ? "Wait for the reply to finish, or press Stop..."
      : "Another conversation is still replying...";
  }
  if (s.startingModel || s.runtimeState === "starting") return "Starting model...";
  if (s.conversationModelUnregistered) {
    return "This conversation's model is no longer available...";
  }
  if (s.conversationModelFileMissing) return "Restore the model file to continue...";
  if (s.conversationModelInvalid) {
    return "This conversation's model file is invalid. Choose another model...";
  }
  if (s.needsRuntimeReload) return "Load this conversation's model to continue...";
  if (s.loadedModelUnavailable) {
    return "The loaded model is no longer available. Choose another model...";
  }
  switch (s.runtimeState) {
    case "error":
      return s.engineErrorMessage ?? "Omnira's engine is not responding.";
    case "ready":
      return "Message your local model...";
    case "stopped":
      return "Choose a model to start";
    default: {
      const exhaustive: never = s.runtimeState;
      return exhaustive;
    }
  }
}

/**
 * Model to restart after the engine failed: the conversation's own model
 * first, then the last model the engine reported. The core clears
 * `model_id` on error, so the caller remembers the last one it saw.
 */
export function retryModelId(
  models: Pick<ModelEntry, "id" | "status">[],
  conversationModelId: string | null,
  lastRuntimeModelId: string | null,
): string | null {
  for (const id of [conversationModelId, lastRuntimeModelId]) {
    if (id && models.some((m) => m.id === id && m.status === "ok")) return id;
  }
  return null;
}

/** Whether the Send button may submit `draft`. */
export function canSubmitDraft(input: ComposerLockInput, draft: string): boolean {
  return !composerDisabled(input) && draft.trim().length > 0;
}
