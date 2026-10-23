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

/** Whether the Send button may submit `draft`. */
export function canSubmitDraft(input: ComposerLockInput, draft: string): boolean {
  return !composerDisabled(input) && draft.trim().length > 0;
}
