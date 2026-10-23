/**
 * Pure presentation decisions for the Chat screen, kept out of the component
 * so they can be unit tested without mounting React.
 */

export interface ComposerLockInput {
  /** `sendBlocker(...) === null` for the latest runtime snapshot. */
  canSend: boolean;
  generating: boolean;
  /** A send is between the click and the stream starting. */
  sending: boolean;
  /** The user picked a model and its engine is being started. */
  startingModel: boolean;
}

/** Whether the composer textarea must refuse input right now. */
export function composerDisabled({
  canSend,
  generating,
  sending,
  startingModel,
}: ComposerLockInput): boolean {
  return !canSend || generating || sending || startingModel;
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

/** Whether the Send button may submit `draft`. */
export function canSubmitDraft(input: ComposerLockInput, draft: string): boolean {
  return !composerDisabled(input) && draft.trim().length > 0;
}
